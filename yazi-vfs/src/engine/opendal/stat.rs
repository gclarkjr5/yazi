use std::time::SystemTime;

use yazi_binding::time::Time;
use yazi_fs::stat::{StatKind, StatMode};

pub(super) struct Stat(pub(super) yazi_fs::stat::Stat);

impl Stat {
	pub(super) fn dir(name: &str) -> Self { Self::new(name, StatMode::T_DIR, 0, None) }

	fn new(name: &str, r#type: StatMode, len: u64, mtime: Option<Time>) -> Self {
		// Object storage has no permissions; report the usual 644/755 so that local copies stay writable
		let perm = StatMode::U_READ | StatMode::U_WRITE | StatMode::G_READ | StatMode::O_READ;
		let exec = if r#type == StatMode::T_DIR {
			StatMode::U_EXEC | StatMode::G_EXEC | StatMode::O_EXEC
		} else {
			StatMode::empty()
		};

		Self(yazi_fs::stat::Stat {
			kind: if name.starts_with('.') { StatKind::HIDDEN } else { StatKind::empty() },
			mode: r#type | perm | exec,
			len,
			mtime,
			..Default::default()
		})
	}
}

impl From<(&str, &::opendal::Metadata)> for Stat {
	fn from((name, meta): (&str, &::opendal::Metadata)) -> Self {
		let mtime = meta.last_modified().map(SystemTime::from).map(Time::from);

		if meta.is_dir() {
			Self::new(name, StatMode::T_DIR, 0, mtime)
		} else {
			Self::new(name, StatMode::T_FILE, meta.content_length(), mtime)
		}
	}
}
