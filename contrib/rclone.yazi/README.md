# rclone.yazi

Browse any [rclone](https://rclone.org) remote in Yazi as a virtual filesystem: S3, Google Drive, Dropbox, OneDrive, Box, WebDAV, Backblaze B2, and [~70 more](https://rclone.org/overview/).

Built on Yazi's custom VFS provider API (`kind = "scope"`, `M:provide(job)`).

## What works

| Operation | Status |
|---|---|
| List directories | ✅ streamed from `rclone lsjson` |
| File size / modified time | ✅ |
| Preview, open, copy files to local (`y` then `p`) | ✅ ranged reads via `rclone cat --offset` |
| Write, rename, delete, mkdir | ❌ read-only for now; returns `ReadOnlyFilesystem` / `Unsupported` |

## Setup

1. Install rclone and configure a remote (`brew install rclone && rclone config`). Check it works with `rclone lsd gdrive:`.

2. Install the plugin:

   ```sh
   mkdir -p ~/.config/yazi/plugins
   cp -r rclone.yazi ~/.config/yazi/plugins/
   ```

3. Register one VFS service per remote in `~/.config/yazi/vfs.toml`:

   ```toml
   [rclone.gdrive]
   kind   = "scope"
   run    = "rclone"
   # remote = "gdrive:"            # default: "<domain>:"
   # remote = "s3:my-bucket/photos" # start inside a bucket/sub-path
   # bin    = "/opt/homebrew/bin/rclone"
   ```

4. Optional key binding in `~/.config/yazi/keymap.toml`:

   ```toml
   [[mgr.prepend_keymap]]
   on   = [ "g", "r" ]
   run  = "plugin rclone gdrive"
   desc = "Go to Google Drive"
   ```

Then press `gr`, or type `g` `<Space>` (cd interactively) and enter `rclone://gdrive//`.

## How it maps

`rclone://<domain>//<path>` → `rclone <remote><path>`, for example `rclone://gdrive//Photos/2024` → `gdrive:Photos/2024`.

Each VFS operation starts a fresh `rclone` process. That's fine for browsing, but listings cost one API round-trip each.

## Possible next steps

- Writes: `Open` with `demand.write` → `rclone rcat`; `Rename` → `moveto`; `RemoveFile` → `deletefile`; `CreateDir` → `mkdir`.
- Server-side copy within one remote (`copy_to` capability → `rclone copyto`).
- Use a long-running `rclone rcd` and its HTTP API instead of a process per operation.
