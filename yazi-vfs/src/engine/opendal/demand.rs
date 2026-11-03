use std::io;

use yazi_fs::engine::{Attrs, Engine, FileBuilder};
use yazi_shared::url::AsUrl;

use super::{Opendal, read_only};

#[derive(Clone, Copy, Default)]
pub struct Demand(yazi_fs::engine::Demand);

impl FileBuilder for Demand {
	type File = super::RwFile;

	fn append(&mut self, append: bool) -> &mut Self {
		self.0.append = append;
		self
	}

	fn attrs(&mut self, attrs: Attrs) -> &mut Self {
		self.0.attrs = attrs;
		self
	}

	fn create(&mut self, create: bool) -> &mut Self {
		self.0.create = create;
		self
	}

	fn create_new(&mut self, create_new: bool) -> &mut Self {
		self.0.create_new = create_new;
		self
	}

	async fn open<U>(&self, url: U) -> io::Result<Self::File>
	where
		U: AsUrl,
	{
		let d = self.0;
		if d.write || d.append || d.create || d.create_new || d.truncate {
			return Err(read_only());
		}

		let engine = Opendal::new(url.as_url()).await?;
		let stat = engine.metadata().await?;
		if stat.is_dir() {
			return Err(io::ErrorKind::IsADirectory.into());
		}

		let reader = engine.op.reader(engine.key()?).await?;
		super::RwFile::new(reader, stat, url.as_url()).await
	}

	fn read(&mut self, read: bool) -> &mut Self {
		self.0.read = read;
		self
	}

	fn truncate(&mut self, truncate: bool) -> &mut Self {
		self.0.truncate = truncate;
		self
	}

	fn write(&mut self, write: bool) -> &mut Self {
		self.0.write = write;
		self
	}
}
