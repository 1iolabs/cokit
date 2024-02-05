use derive_more::{From, Into};
use libipld::{Cid, Ipld};
use serde::{Deserialize, Serialize};
use std::{
	cmp::Ordering,
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
};

/// Tag Value
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, From, Serialize, Deserialize)]
#[serde(into = "Ipld", from = "Ipld")]
pub enum TagValue {
	/// Represents the absence of a value or the value undefined.
	Null,
	/// Represents a boolean value.
	#[from]
	Bool(bool),
	/// Represents an integer.
	#[from(types(i8, i16, i32, i64))]
	Integer(i128),
	/// Represents a floating point value.
	Float(TotalFloat),
	/// Represents an UTF-8 string.
	#[from]
	String(String),
	/// Represents a sequence of bytes.
	#[from]
	Bytes(Vec<u8>),
	/// Represents a list.
	#[from]
	List(Vec<TagValue>),
	/// Represents a map of strings.
	#[from]
	Map(BTreeMap<String, TagValue>),
	/// Represents an IPLD Link structure, implemented with Cid's (Content Identifiers)
	/// For more information see: https://ipld.io/docs/data-model/kinds/#link-kind
	#[from]
	Link(Cid),
}
impl Into<Ipld> for TagValue {
	fn into(self) -> Ipld {
		match self {
			TagValue::Null => Ipld::Null,
			TagValue::Bool(i) => Ipld::Bool(i),
			TagValue::Integer(i) => Ipld::Integer(i),
			TagValue::Float(i) => Ipld::Float(i.0),
			TagValue::String(i) => Ipld::String(i),
			TagValue::Bytes(i) => Ipld::Bytes(i),
			TagValue::List(i) => Ipld::List(i.into_iter().map(|e| e.into()).collect()),
			TagValue::Map(i) => Ipld::Map(i.into_iter().map(|(k, v)| (k, v.into())).collect()),
			TagValue::Link(i) => Ipld::Link(i),
		}
	}
}
impl From<Ipld> for TagValue {
	fn from(value: Ipld) -> Self {
		match value {
			Ipld::Null => TagValue::Null,
			Ipld::Bool(i) => TagValue::Bool(i),
			Ipld::Integer(i) => TagValue::Integer(i),
			Ipld::Float(i) => TagValue::Float(i.into()),
			Ipld::String(i) => TagValue::String(i),
			Ipld::Bytes(i) => TagValue::Bytes(i),
			Ipld::List(i) => TagValue::List(i.into_iter().map(|e| e.into()).collect()),
			Ipld::Map(i) => TagValue::Map(i.into_iter().map(|(k, v)| (k, v.into())).collect()),
			Ipld::Link(i) => TagValue::Link(i),
		}
	}
}

/// Tag. Represents a generic metadata/configuration key value pair.
pub type Tag = (String, TagValue);

/// Tags.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, From, Serialize, Deserialize)]
pub struct Tags {
	tags: BTreeSet<Tag>,
}
impl Tags {
	pub fn new() -> Self {
		Self { tags: Default::default() }
	}

	/// Insert tag.
	pub fn insert(&mut self, tag: Tag) {
		self.tags.insert(tag);
	}

	/// Remove tag.
	pub fn remove(&mut self, tag: &Tag) {
		self.tags.remove(tag);
	}

	/// Insert mutiple tags.
	pub fn append(&mut self, tags: &mut Tags) {
		self.tags.append(&mut tags.tags);
	}

	/// Remove specified tags.
	/// If no tags are specified all tags will be removed.
	pub fn clear(&mut self, tags: Option<&Tags>) {
		match tags {
			Some(tags) =>
				for tag in tags.tags.iter() {
					self.tags.remove(tag);
				},
			None => self.tags.clear(),
		}
	}
}
impl Debug for Tags {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut s = f.debug_struct("Tags");
		for (key, value) in self.tags.iter() {
			s.field(key, value);
		}
		s.finish()
	}
}

/// Tags match pattern.
///
/// Todo: implement
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, From, Serialize, Deserialize)]
pub struct TagsPattern {}

/// f64 float wich uses total order from IEEE 754 (2008 revision).
#[derive(Debug, Clone, Copy, From, Into)]
pub struct TotalFloat(f64);
impl PartialEq for TotalFloat {
	fn eq(&self, other: &Self) -> bool {
		self.0.total_cmp(&other.0) == Ordering::Equal
	}
}
impl Eq for TotalFloat {}
impl PartialOrd for TotalFloat {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.0.total_cmp(&other.0))
	}
}
impl Ord for TotalFloat {
	fn cmp(&self, other: &Self) -> Ordering {
		self.0.total_cmp(&other.0)
	}
}
