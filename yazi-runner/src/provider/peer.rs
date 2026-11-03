use std::sync::Arc;

use mlua::{IntoLua, Lua, Value};
use yazi_config::{VFS, vfs::{ServiceLua, ServiceOpendal, ServiceSftp}};
use yazi_shared::{auth::AuthKind, url::Url};

pub enum ProvidePeer {
	Local,
	Lua(Arc<ServiceLua>),
	Sftp(Arc<ServiceSftp>),
	Opendal(Arc<ServiceOpendal>),
}

impl ProvidePeer {
	pub fn new(url: Url<'_>) -> std::io::Result<Self> {
		match url.kind() {
			AuthKind::Regular => Ok(Self::Local),
			AuthKind::Sftp => VFS.service(url.auth()).map(Self::Sftp),
			AuthKind::Opendal => VFS.service(url.auth()).map(Self::Opendal),
			AuthKind::Mount | AuthKind::Hub | AuthKind::Scope | AuthKind::View => {
				VFS.service(url.auth()).map(Self::Lua)
			}
		}
	}
}

impl IntoLua for ProvidePeer {
	fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
		match self {
			Self::Local => Ok(Value::Nil),
			Self::Lua(service) => service.into_lua(lua),
			Self::Sftp(service) => service.into_lua(lua),
			Self::Opendal(service) => service.into_lua(lua),
		}
	}
}
