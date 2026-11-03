use std::{io, sync::Arc};

use futures::TryStreamExt;
use yazi_fs::{engine::{DirReader, FileHolder}, file::File};
use yazi_shared::{path::PathBufDyn, strand::StrandCow, url::{UrlBuf, UrlLike}};

use super::Stat;
use crate::VfsFile;

pub struct ReadDir {
	pub(super) dir:    Arc<UrlBuf>,
	pub(super) lister: ::opendal::Lister,
	pub(super) prefix: String,
}

impl DirReader for ReadDir {
	type Entry = DirEntry;

	async fn next(&mut self) -> io::Result<Option<Self::Entry>> {
		while let Some(entry) = self.lister.try_next().await? {
			// The lister yields the listed directory itself as well
			if entry.path() == self.prefix {
				continue;
			}

			let name = entry.name().trim_end_matches('/');
			let stat = Stat::from((name, entry.metadata())).0;
			return Ok(Some(DirEntry { dir: self.dir.clone(), name: name.to_owned(), stat }));
		}

		Ok(None)
	}
}

// --- Entry
pub struct DirEntry {
	dir:  Arc<UrlBuf>,
	name: String,
	stat: yazi_fs::stat::Stat,
}

impl FileHolder for DirEntry {
	async fn file(&self) -> io::Result<File> { Ok(File::from_follow(self.url(), self.stat).await) }

	async fn file_type(&self) -> io::Result<yazi_fs::stat::StatType> { Ok(*self.stat.mode) }

	async fn metadata(&self) -> io::Result<yazi_fs::stat::Stat> { Ok(self.stat) }

	fn name(&self) -> StrandCow<'_> { self.name.as_str().into() }

	fn path(&self) -> PathBufDyn { self.url().loc().into() }

	fn url(&self) -> UrlBuf {
		self.dir.try_join(&self.name).expect("entry name is a valid component of the OpenDAL URL")
	}
}
