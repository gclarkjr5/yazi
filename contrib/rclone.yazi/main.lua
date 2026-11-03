--- Browse any rclone remote (S3, Google Drive, Dropbox, OneDrive, WebDAV, ...) as a Yazi VFS.
---
--- vfs.toml:
---   [rclone.gdrive]          # URL: rclone://gdrive//
---   kind   = "scope"
---   run    = "rclone"
---   remote = "gdrive:"       # optional, defaults to "<domain>:"; may include a sub-path, e.g. "s3:bucket/prefix"
---   bin    = "rclone"        # optional, path to the rclone binary

local M = {}

local CHUNK = 256 * 1024
local DIR_MODE = tonumber("40755", 8)
local FILE_MODE = tonumber("100644", 8)

local function nfound(message) return Error.fs { kind = "NotFound", message = message } end

local function bin(job) return job.opts.bin or "rclone" end

-- Map a VFS URL to an rclone path, e.g. rclone://gdrive//a/b -> gdrive:a/b
local function target(job, url)
	local remote = job.opts.remote or (url.spec.domain .. ":")
	local rel = tostring(url.path):gsub("^/+", ""):gsub("/+$", "")
	if rel == "" or rel == "." then
		return remote
	elseif remote:match("[:/]$") then
		return remote .. rel
	else
		return remote .. "/" .. rel
	end
end

local function is_root(url)
	local path = tostring(url.path)
	return path == "/" or path == "" or path == "."
end

local function stat(ent)
	return Stat {
		mode = ent.IsDir and DIR_MODE or FILE_MODE,
		len = math.max(ent.Size or 0, 0),
		-- Strip fractional seconds; rclone emits nanosecond precision.
		mtime = ent.ModTime and ent.ModTime:gsub("%.%d+", "") or nil,
	}
end

local function file(url, ent)
	local s = stat(ent)
	return File { url = url, stat = s, lstat = s }
end

local function run(job, args)
	local output, err = Command(bin(job))
		:arg(args)
		:stdin(Command.NULL)
		:stdout(Command.PIPED)
		:stderr(Command.PIPED)
		:output()

	if not output then
		return nil, Err("Failed to run `%s`: %s", bin(job), err)
	elseif output.status.success then
		return output.stdout
	end

	local message = output.stderr:gsub("[^\n]*NOTICE:[^\n]*\n?", ""):gsub("%s+$", "")
	local code = output.status.code
	if code == 3 or code == 4 then
		return nil, nfound(message)
	end
	return nil, Err("rclone exited with %s: %s", code, message)
end

local function lookup(job, url)
	if is_root(url) then
		return { IsDir = true, Size = 0 }
	end

	local out, err = run(job, { "lsjson", "--stat", "--no-mimetype", target(job, url) })
	if not out then
		return nil, err
	end

	local ent, err = ya.json_decode(out)
	if not ent then
		return nil, err
	end
	return ent
end

local function absolute(url)
	if url.is_absolute then
		return fs.clean_url(url)
	end
	local spec = url.spec
	return fs.clean_url(Url(string.format("%s://%s//%s", spec.scheme, spec.domain, tostring(url.path))))
end

local function read_dir(job)
	return ya.co(function()
		local child, err = Command(bin(job))
			:arg({ "lsjson", "--no-mimetype", target(job, job.url) })
			:stdin(Command.NULL)
			:stdout(Command.PIPED)
			:stderr(Command.NULL)
			:spawn()
		if not child then
			return nil, Err("Failed to run `%s`: %s", bin(job), err)
		end

		local child <close> = child
		while true do
			local line, event = child:read_line()
			if event ~= 0 then
				break
			end

			-- rclone prints one entry per line: `{...},` between `[` and `]`
			local json = line:match("^%s*({.*})%s*,?%s*$")
			local ent = json and ya.json_decode(json)
			if ent then
				coroutine.yield(file(job.url:join(ent.Name), ent))
			end
		end

		local status = child:wait()
		if status and status.code == 3 then
			return nil, nfound("directory not found")
		elseif status and not status.success then
			return nil, Err("rclone lsjson exited with %s", status.code)
		end
	end)
end

local function read(job)
	return ya.co(function()
		local child, err = Command(bin(job))
			:arg({ "cat", "--offset", tostring(job.offset.value), target(job, job.url) })
			:stdin(Command.NULL)
			:stdout(Command.PIPED)
			:stderr(Command.NULL)
			:spawn()
		if not child then
			return nil, Err("Failed to run `%s`: %s", bin(job), err)
		end

		local child <close> = child
		while true do
			local data, event = child:read(CHUNK)
			if event ~= 0 then
				break
			end
			coroutine.yield(data)
		end

		local status = child:wait()
		if status and not status.success then
			return nil, Err("rclone cat exited with %s", status.code)
		end
	end)
end

function M:entry(job)
	local remote = job.args[1]
	if remote then
		ya.emit("cd", { Url(string.format("rclone://%s//", remote)) })
	else
		ya.notify { title = "rclone", content = "Usage: plugin rclone <domain>", timeout = 5, level = "warn" }
	end
end

function M:provide(job)
	local op = job.op
	if op == "Capabilities" then
		return {}
	elseif op == "Absolute" or op == "Canonicalize" then
		return absolute(job.url)
	elseif op == "Casefold" then
		return job.url
	elseif op == "Metadata" or op == "SymlinkMetadata" then
		local ent, err = lookup(job, job.url)
		return ent and stat(ent), err
	elseif op == "File" or op == "Revalidate" then
		local url = job.url or job.file.url
		local ent, err = lookup(job, url)
		return ent and file(url, ent), err
	elseif op == "ReadDir" then
		return read_dir(job)
	elseif op == "Open" then
		if job.demand.write or job.demand.append or job.demand.create or job.demand.truncate then
			return nil, Error.fs { kind = "ReadOnlyFilesystem", message = "rclone VFS is read-only" }
		end
		return { id = 1, random = "read", state = target(job, job.url) }
	elseif op == "Read" then
		return read(job)
	elseif op == "Close" then
		return true
	end

	return false, Error.fs { kind = "Unsupported", message = string.format("rclone VFS does not support %s", op) }
end

return M
