use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// A (serializable) typed link.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
#[serde(into = "Cid", from = "Cid")]
pub struct Link<T: Clone> {
	_type: PhantomData<T>,
	cid: Cid,
}
impl<T: Clone> Link<T> {
	pub fn new(cid: Cid) -> Self {
		Self { cid, _type: Default::default() }
	}
}
impl<T: Clone> Into<Cid> for Link<T> {
	fn into(self) -> Cid {
		self.cid
	}
}
impl<T: Clone> From<Cid> for Link<T> {
	fn from(value: Cid) -> Self {
		Self::new(value)
	}
}
impl<T: Clone> AsRef<Cid> for Link<T> {
	fn as_ref(&self) -> &Cid {
		&self.cid
	}
}
impl<T: Clone> Linkable<T> for Link<T> {
	fn cid(&self) -> &Cid {
		&self.cid
	}
}

pub trait Linkable<T> {
	fn cid(&self) -> &Cid;
}
