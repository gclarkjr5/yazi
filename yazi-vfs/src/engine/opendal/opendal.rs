use std::{io, sync::{Arc, LazyLock}};

use hashbrown::HashMap;
use parking_lot::Mutex;
use typed_path::{UnixPath, UnixPathBuf};
use yazi_config::{VFS, vfs::ServiceOpendal};
use yazi_fs::{engine::{Attrs, Capabilities, Engine, Transit}, remover::Removal};
use yazi_shared::{auth::AuthKind, path::{DynPath, PathBufDyn}, strand::AsStrand, url::{AsUrl, Url, UrlBuf, UrlCow, UrlLike}};

use super::Stat;

static OPERATORS: LazyLock<Mutex<HashMap<Arc<ServiceOpendal>, ::opendal::Operator>>> =
	LazyLock::new(Default::default);

pub struct Opendal<'a> {
	url:             Url<'a>,
	pub(super) path: &'a UnixPath,

	config:        Arc<ServiceOpendal>,
	pub(super) op: ::opendal::Operator,
}

impl AsUrl for Opendal<'_> {
	fn as_url(&self) -> Url<'_> { self.url }
}

impl UrlLike for Opendal<'_> {}

impl<'a> Engine for Opendal<'a> {
	type Demand = super::Demand;
	type File = super::RwFile;
	type Me<'b> = Opendal<'b>;
	type ReadDir = super::ReadDir;
	type UrlCow = UrlCow<'a>;

	async fn absolute(&self) -> io::Result<Self::UrlCow> {
		Ok(if let Some(u) = crate::engine::try_absolute_impl(self.url) {
			u
		} else {
			self.canonicalize().await?.into()
		})
	}

	async fn canonicalize(&self) -> io::Result<UrlBuf> {
		Ok(UrlBuf::Unix {
			loc:  UnixPathBuf::from("/").join(self.path).normalize().into(),
			auth: self.config.auth.clone(),
		})
	}

	async fn capabilities(&self) -> io::Result<Capabilities> {
		Ok(Capabilities::for_kind(AuthKind::Opendal))
	}

	async fn casefold(&self) -> io::Result<UrlBuf> { Ok(self.to_url()) }

	async fn copy_to(&self, _to: Url<'_>, _attrs: Attrs) -> io::Result<Transit> {
		Ok(Transit::unsupported())
	}

	async fn create_dir(&self) -> io::Result<()> { Err(read_only()) }

	async fn create_file(&self) -> io::Result<()> { Err(read_only()) }

	async fn create_file_new(&self) -> io::Result<()> { Err(read_only()) }

	async fn hard_link<P>(&self, _to: P) -> io::Result<()>
	where
		P: DynPath,
	{
		Err(io::ErrorKind::Unsupported.into())
	}

	async fn metadata(&self) -> io::Result<yazi_fs::stat::Stat> {
		let name = self.path.file_name().map(str::from_utf8).transpose().ok().flatten();
		let Some(name) = name else {
			return Ok(Stat::dir("").0);
		};

		let key = self.key()?;
		let meta = match self.op.stat(key).await {
			Err(e) if e.kind() == ::opendal::ErrorKind::NotFound => {
				self.op.stat(&format!("{key}/")).await?
			}
			result => result?,
		};

		Ok(Stat::from((name, &meta)).0)
	}

	async fn new<'b>(url: Url<'b>) -> io::Result<Self::Me<'b>> {
		let Url::Unix { loc, auth } = url else {
			return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Not an OpenDAL URL: {url}")));
		};

		let config: Arc<ServiceOpendal> = VFS.service(auth)?;
		let op = Self::operator(&config)?;
		Ok(Self::Me { url, path: loc.as_inner(), config, op })
	}

	async fn read_dir(self) -> io::Result<Self::ReadDir> {
		let key = self.key()?;
		let prefix = if key.is_empty() { "/".to_owned() } else { format!("{key}/") };

		Ok(Self::ReadDir {
			dir:    Arc::new(self.to_url()),
			lister: self.op.lister(&prefix).await?,
			prefix,
		})
	}

	async fn read_link(&self) -> io::Result<PathBufDyn> { Err(io::ErrorKind::Unsupported.into()) }

	async fn remove_dir(&self) -> io::Result<()> { Err(read_only()) }

	async fn remove_dir_all(&self) -> io::Result<Removal> { Err(read_only()) }

	async fn remove_file(&self) -> io::Result<()> { Err(read_only()) }

	async fn rename<P>(&self, _to: P) -> io::Result<()>
	where
		P: DynPath,
	{
		Err(read_only())
	}

	async fn set_attrs(&self, _attrs: Attrs) -> io::Result<()> { Err(read_only()) }

	async fn symlink<S, F>(&self, _original: S, _is_dir: F) -> io::Result<()>
	where
		S: AsStrand,
		F: AsyncFnOnce() -> io::Result<bool>,
	{
		Err(io::ErrorKind::Unsupported.into())
	}

	async fn symlink_metadata(&self) -> io::Result<yazi_fs::stat::Stat> { self.metadata().await }

	async fn trash(&self) -> io::Result<()> { Err(io::ErrorKind::Unsupported.into()) }
}

impl Opendal<'_> {
	/// The OpenDAL object key of this URL: relative to the service root, without
	/// leading or trailing slashes, and empty for the root itself.
	pub(super) fn key(&self) -> io::Result<&str> {
		let key = str::from_utf8(self.path.as_bytes())
			.map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "OpenDAL paths must be UTF-8"))?;

		Ok(match key.trim_matches('/') {
			"." => "",
			key => key.strip_prefix("./").unwrap_or(key),
		})
	}

	fn operator(config: &Arc<ServiceOpendal>) -> io::Result<::opendal::Operator> {
		let mut operators = OPERATORS.lock();
		if let Some(op) = operators.get(config) {
			return Ok(op.clone());
		}

		let op = ::opendal::Operator::via_iter(&config.service, config.options.clone())?;
		operators.insert(config.clone(), op.clone());
		Ok(op)
	}
}

pub(crate) fn read_only() -> io::Error {
	io::Error::new(io::ErrorKind::ReadOnlyFilesystem, "OpenDAL VFS is read-only")
}
