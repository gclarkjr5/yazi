use std::{collections::BTreeMap, ops::Deref};

use mlua::UserData;
use serde::Deserialize;
use yazi_shared::auth::{Auth, AuthArc};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct ServiceOpendal {
	#[serde(skip)]
	pub auth:    AuthArc,
	pub service: String,
	#[serde(flatten)]
	pub options: BTreeMap<String, String>,
}

impl Deref for ServiceOpendal {
	type Target = Auth;

	fn deref(&self) -> &Self::Target { &self.auth }
}

impl UserData for ServiceOpendal {}
