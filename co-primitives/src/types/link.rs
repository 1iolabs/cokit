// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::CoCid;
use cid::Cid;
use either::Either;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
	any::type_name,
	fmt::{Debug, Display},
	hash::Hash,
	marker::PhantomData,
};

pub trait Linkable<T> {
	fn value(&self) -> Either<Cid, T>;
}

/// A (serializable) typed link.
#[derive(Serialize, Deserialize)]
#[serde(into = "Cid", from = "Cid")]
pub struct Link<T> {
	#[serde(skip)]
	_type: PhantomData<T>,
	cid: Cid,
}
impl<T> Ord for Link<T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cid.cmp(&other.cid)
	}
}
impl<T> Eq for Link<T> {}
impl<T> PartialOrd for Link<T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl<T> PartialEq for Link<T> {
	fn eq(&self, other: &Self) -> bool {
		self.cid == other.cid
	}
}
impl<T> Hash for Link<T> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		std::hash::Hash::hash(&self.cid, state);
	}
}
impl<T> Copy for Link<T> {}
impl<T> Linkable<T> for Link<T> {
	fn value(&self) -> Either<Cid, T> {
		Either::Left(self.cid)
	}
}
impl<T> Link<T> {
	pub fn new(cid: Cid) -> Self {
		Self { cid, _type: Default::default() }
	}

	pub fn cid(&self) -> &Cid {
		&self.cid
	}
}
impl<T> Clone for Link<T> {
	fn clone(&self) -> Self {
		*self
	}
}
impl<T> From<Link<T>> for Cid {
	fn from(val: Link<T>) -> Self {
		val.cid
	}
}
impl<T> From<Link<T>> for Option<Cid> {
	fn from(val: Link<T>) -> Self {
		Some(val.cid)
	}
}
impl<T> From<Cid> for Link<T> {
	fn from(value: Cid) -> Self {
		Self::new(value)
	}
}
impl<T> From<&Cid> for Link<T> {
	fn from(value: &Cid) -> Self {
		Self::new(*value)
	}
}
impl<T> AsRef<Cid> for Link<T> {
	fn as_ref(&self) -> &Cid {
		&self.cid
	}
}
impl<T> Debug for Link<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Link({}: {})", type_name::<T>(), self.cid)
	}
}
impl<T> Display for Link<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Link({}: {})", type_name::<T>(), self.cid)
	}
}

/// A (serializable) typed link.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(into = "Option<Cid>", from = "Option<Cid>")]
pub struct OptionLink<T> {
	#[serde(skip)]
	_type: PhantomData<T>,
	#[schemars(with = "CoCid")]
	cid: Option<Cid>,
}
impl<T> Default for OptionLink<T> {
	fn default() -> Self {
		Self { _type: Default::default(), cid: Default::default() }
	}
}
impl<T> PartialEq for OptionLink<T> {
	fn eq(&self, other: &Self) -> bool {
		self.cid == other.cid
	}
}
impl<T> Hash for OptionLink<T> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.cid.hash(state);
	}
}
impl<T> Ord for OptionLink<T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cid.cmp(&other.cid)
	}
}
impl<T> Eq for OptionLink<T> {}
impl<T> PartialOrd for OptionLink<T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl<T: Default> Linkable<T> for OptionLink<T> {
	fn value(&self) -> Either<Cid, T> {
		match self.cid {
			Some(cid) => Either::Left(cid),
			None => Either::Right(T::default()),
		}
	}
}
impl<T> Copy for OptionLink<T> {}
impl<T> OptionLink<T> {
	pub fn new(cid: Option<Cid>) -> Self {
		Self { cid, _type: Default::default() }
	}

	pub fn none() -> Self {
		Self { cid: None, _type: Default::default() }
	}

	pub fn is_none(&self) -> bool {
		self.cid.is_none()
	}

	pub fn cid(&self) -> &Option<Cid> {
		&self.cid
	}

	pub fn set(&mut self, cid: Option<Cid>) {
		self.cid = cid;
	}

	pub fn link(&self) -> Option<Link<T>> {
		self.cid.map(Link::new)
	}

	pub fn unwrap(&self) -> Link<T> {
		#[allow(clippy::unwrap_used)]
		Link::new(self.cid.unwrap())
	}

	pub fn expect(&self, message: &str) -> Link<T> {
		Link::new(self.cid.expect(message))
	}
}
impl<T> Clone for OptionLink<T> {
	fn clone(&self) -> Self {
		*self
	}
}
impl<T> From<OptionLink<T>> for Option<Cid> {
	fn from(val: OptionLink<T>) -> Self {
		val.cid
	}
}
impl<T> From<Option<Cid>> for OptionLink<T> {
	fn from(value: Option<Cid>) -> Self {
		Self::new(value)
	}
}
impl<T> From<Link<T>> for OptionLink<T> {
	fn from(value: Link<T>) -> Self {
		Self::new(Some(value.into()))
	}
}
impl<T> From<&Option<Cid>> for OptionLink<T> {
	fn from(value: &Option<Cid>) -> Self {
		Self::new(*value)
	}
}
impl<T> From<Cid> for OptionLink<T> {
	fn from(value: Cid) -> Self {
		Self::new(Some(value))
	}
}
impl<T> From<&Cid> for OptionLink<T> {
	fn from(value: &Cid) -> Self {
		Self::new(Some(*value))
	}
}
impl<T> AsRef<Option<Cid>> for OptionLink<T> {
	fn as_ref(&self) -> &Option<Cid> {
		&self.cid
	}
}
impl<T> Debug for OptionLink<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Link({}: {:?})", type_name::<T>(), self.cid)
	}
}
impl<T> Display for OptionLink<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self.cid {
			Some(cid) => write!(f, "{} ({})", cid, type_name::<T>()),
			None => write!(f, "None ({})", type_name::<T>()),
		}
	}
}
