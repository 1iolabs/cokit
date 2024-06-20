use either::Either;
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::{
	any::type_name,
	fmt::{Debug, Display},
	marker::PhantomData,
};

pub trait Linkable<T> {
	fn value(&self) -> Either<Cid, T>;
}

/// A (serializable) typed link.
#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
#[serde(into = "Cid", from = "Cid")]
pub struct Link<T> {
	#[serde(skip)]
	_type: PhantomData<T>,
	cid: Cid,
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
		Self { _type: self._type.clone(), cid: self.cid.clone() }
	}
}
impl<T> Into<Cid> for Link<T> {
	fn into(self) -> Cid {
		self.cid
	}
}
impl<T> Into<Option<Cid>> for Link<T> {
	fn into(self) -> Option<Cid> {
		Some(self.cid)
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
#[derive(Default, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
#[serde(into = "Option<Cid>", from = "Option<Cid>")]
pub struct OptionLink<T: Default> {
	#[serde(skip)]
	_type: PhantomData<T>,
	cid: Option<Cid>,
}
impl<T: Default> Linkable<T> for OptionLink<T> {
	fn value(&self) -> Either<Cid, T> {
		match self.cid {
			Some(cid) => Either::Left(cid),
			None => Either::Right(T::default()),
		}
	}
}
impl<T: Default> Copy for OptionLink<T> {}
impl<T: Default> OptionLink<T> {
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
}
impl<T: Default> Clone for OptionLink<T> {
	fn clone(&self) -> Self {
		Self { _type: self._type.clone(), cid: self.cid.clone() }
	}
}
impl<T: Default> Into<Option<Cid>> for OptionLink<T> {
	fn into(self) -> Option<Cid> {
		self.cid
	}
}
impl<T: Default> From<Option<Cid>> for OptionLink<T> {
	fn from(value: Option<Cid>) -> Self {
		Self::new(value)
	}
}
impl<T: Default> From<&Option<Cid>> for OptionLink<T> {
	fn from(value: &Option<Cid>) -> Self {
		Self::new(*value)
	}
}
impl<T: Default> From<Cid> for OptionLink<T> {
	fn from(value: Cid) -> Self {
		Self::new(Some(value))
	}
}
impl<T: Default> AsRef<Option<Cid>> for OptionLink<T> {
	fn as_ref(&self) -> &Option<Cid> {
		&self.cid
	}
}
impl<T: Default> Debug for OptionLink<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Link({}: {:?})", type_name::<T>(), self.cid)
	}
}
impl<T: Default> Display for OptionLink<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self.cid {
			Some(cid) => write!(f, "{} ({})", cid.to_string(), type_name::<T>()),
			None => write!(f, "None ({})", type_name::<T>()),
		}
	}
}
