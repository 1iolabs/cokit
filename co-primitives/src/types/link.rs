use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// A (serializable) typed link.
#[derive(Debug, Copy, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
#[serde(into = "Cid", from = "Cid")]
pub struct Link<T> {
	#[serde(skip)]
	_type: PhantomData<T>,
	cid: Cid,
}
impl<T> Link<T> {
	pub fn new(cid: Cid) -> Self {
		Self { cid, _type: Default::default() }
	}
}
impl<T> Clone for Link<T> {
	fn clone(&self) -> Self {
		Self { _type: self._type.clone(), cid: self.cid.clone() }
	}
}
impl<T> Into<Cid> for Link<T> {
	fn into(self) -> Cid {
		self.cid
	}
}
impl<T> From<Cid> for Link<T> {
	fn from(value: Cid) -> Self {
		Self::new(value)
	}
}
impl<T> AsRef<Cid> for Link<T> {
	fn as_ref(&self) -> &Cid {
		&self.cid
	}
}
impl<T> Linkable<T> for Link<T> {
	fn cid(&self) -> &Cid {
		&self.cid
	}
}

pub trait Linkable<T> {
	fn cid(&self) -> &Cid;
}
