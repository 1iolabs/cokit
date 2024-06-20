use crate::TotalFloat64;
use derive_more::From;
use libipld::{Cid, Ipld};
use serde::{Deserialize, Serialize};
use std::{
	borrow::Borrow,
	collections::{BTreeMap, BTreeSet},
	fmt::{Debug, Display},
};

/// Tags inline macro.
///
/// ```
/// use co_primitives::tags;
/// let tags = tags!("hello": "world", "test": 123);
/// println!("tags: {:?}", tags);
/// ```
#[macro_export]
macro_rules! tags{
    ( $($key:tt : $val:expr),* $(,)? ) =>{{
        #[allow(unused_mut)]
        let mut map = $crate::Tags::new();
        $(
            #[allow(unused_parens)]
            let _ = map.insert(($key.to_owned(), $val.to_owned().into()));
        )*
        map
    }};
}

/// Tag inline macro.
///
/// ```
/// use co_primitives::tag;
/// let value = tag!("hello": "world");
/// println!("tag: {:?}", value);
/// ```
#[macro_export]
macro_rules! tag {
	($key:tt : $val:expr) => {{
		let tag: $crate::Tag = ($key.to_owned(), $val.to_owned().into());
		tag
	}};
}

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
	Float(TotalFloat64),
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
impl TagValue {
	/// Access the string value.
	pub fn string(&self) -> Option<&str> {
		match self {
			TagValue::String(s) => Some(s),
			_ => None,
		}
	}
}
impl From<TagValue> for Ipld {
	fn from(val: TagValue) -> Self {
		match val {
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
impl std::fmt::Display for TagValue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TagValue::Null => write!(f, "null"),
			TagValue::Bool(v) => write!(f, "{}", if *v { "true" } else { "false" }),
			TagValue::Integer(i) => write!(f, "{}", i),
			TagValue::Float(v) => write!(f, "{}", v.0),
			TagValue::String(v) => write!(f, "{}", v),
			TagValue::Bytes(v) => write!(f, "{:x?}", v),
			TagValue::List(v) => {
				let mut result = Ok(());
				let mut first = true;
				for value in v.iter() {
					if first {
						first = false;
						result = Ok(write!(f, "{}", value)?)
					} else {
						result = Ok(write!(f, ",{}", value)?)
					}
				}
				result
			},
			TagValue::Map(v) => {
				let mut result = Ok(());
				let mut first = true;
				for (key, value) in v.iter() {
					if first {
						first = false;
						result = Ok(write!(f, "{}={}", key, value)?)
					} else {
						result = Ok(write!(f, ",{}={}", key, value)?)
					}
				}
				result
			},
			TagValue::Link(v) => write!(f, "{}", v),
		}
	}
}

/// Tag. Represents a generic metadata/configuration key value pair.
pub type Tag = (String, TagValue);
impl TagsMatches for Tag {
	fn matches(&self, tags: &Tags) -> bool {
		let expr: TagsExpr = self.clone().into();
		expr.matches(tags)
	}
}

/// Tags.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, From, Serialize, Deserialize)]
pub struct Tags(BTreeSet<Tag>);
impl Tags {
	pub fn new() -> Self {
		Self(Default::default())
	}

	/// Tag count.
	pub fn len(&self) -> usize {
		self.0.len()
	}

	/// No tags?
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Insert tag.
	pub fn insert(&mut self, tag: Tag) {
		self.0.insert(tag);
	}

	/// Remove tag.
	pub fn remove(&mut self, tag: &Tag) {
		self.0.remove(tag);
	}

	/// Insert mutiple tags.
	pub fn append(&mut self, tags: &mut Tags) {
		self.0.append(&mut tags.0);
	}

	/// Insert mutiple tags.
	pub fn extend(&mut self, tags: impl Iterator<Item = Tag>) {
		self.0.extend(tags);
	}

	/// Set tag(s). By removing all tags with the same key before insert.
	pub fn set(&mut self, tags: impl Into<Tags>) {
		for tag in tags.into().into_iter() {
			self.clear_key(&tag.0);
			self.insert(tag);
		}
	}

	/// Remove specified tags.
	/// If no tags are specified all tags will be removed.
	pub fn clear(&mut self, tags: Option<&Tags>) {
		match tags {
			Some(tags) => {
				for tag in tags.0.iter() {
					self.0.remove(tag);
				}
			},
			None => self.0.clear(),
		}
	}

	/// Remove tags with key.
	/// If no tags are specified all tags will be removed.
	pub fn clear_key(&mut self, key: &str) {
		let remove: BTreeSet<Tag> = self.0.iter().filter(|tag| tag.0 == key).cloned().collect();
		for i in remove {
			self.0.remove(&i);
		}
	}

	/// Iterate over tags.
	pub fn iter(&self) -> impl Iterator<Item = &Tag> {
		self.0.iter()
	}

	/// Iterate over tags.
	pub fn into_iter(self) -> impl Iterator<Item = Tag> {
		self.0.into_iter()
	}

	/// Find first tag by key.
	pub fn find_key(&self, key: &str) -> Option<&Tag> {
		self.0.iter().find(|tag| tag.0 == key)
	}

	/// Test against tag expression.
	pub fn matches<M: TagsMatches>(&self, expr: impl Borrow<M>) -> bool {
		expr.borrow().matches(self)
	}

	/// Get first tag value (that is a string) for given key.
	pub fn string(&self, key: &str) -> Option<&str> {
		for (tag_key, tag_value) in self.iter() {
			if key == tag_key {
				match tag_value {
					TagValue::String(v) => return Some(v.as_str()),
					_ => {
						continue;
					},
				}
			}
		}
		None
	}
}
impl Debug for Tags {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut s = f.debug_struct("Tags");
		for (key, value) in self.0.iter() {
			s.field(key, value);
		}
		s.finish()
	}
}
impl Display for Tags {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut result = Ok(());
		let mut first = true;
		for (key, value) in self.0.iter() {
			if first {
				first = false;
				result = Ok(write!(f, "{}: {:?}", key, value)?)
			} else {
				result = Ok(write!(f, ", {}: {:?}", key, value)?)
			}
		}
		result
	}
}
impl From<Tag> for Tags {
	fn from(value: Tag) -> Self {
		let mut tags = Tags::new();
		tags.insert(value);
		tags
	}
}
impl TagsMatches for Tags {
	fn matches(&self, tags: &Tags) -> bool {
		let expr: TagsExpr = self.clone().into();
		expr.matches(tags)
	}
}

/// Tags match pattern.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TagsExpr {
	/// Tests if tag exists.
	#[serde(rename = "$tag")]
	Tag(Tag),
	/// Tests if all patterns evaluate to true.
	#[serde(rename = "$and")]
	And(Vec<TagsExpr>),
	/// Tests if some patterns evaluate to true.
	#[serde(rename = "$or")]
	Or(Vec<TagsExpr>),
	/// PErform logical NOT operation in pattern.
	#[serde(rename = "$not")]
	Not(Box<TagsExpr>),
}
impl TagsMatches for TagsExpr {
	fn matches(&self, tags: &Tags) -> bool {
		match self {
			TagsExpr::Tag(cond_tag) => tags.iter().any(|tag| cond_tag == tag),
			TagsExpr::And(and) => !and.iter().any(|cond| !cond.matches(tags)),
			TagsExpr::Or(or) => or.iter().any(|cond| cond.matches(tags)),
			TagsExpr::Not(not) => !not.matches(tags),
		}
	}
}
impl From<Tag> for TagsExpr {
	fn from(value: Tag) -> Self {
		TagsExpr::Tag(value)
	}
}
impl From<Tags> for TagsExpr {
	fn from(value: Tags) -> Self {
		TagsExpr::And(value.into_iter().map(TagsExpr::Tag).collect())
	}
}

/// Type which can be matched against a list of tags.
pub trait TagsMatches {
	fn matches(&self, tags: &Tags) -> bool;
}

#[cfg(test)]
mod tests {
	use crate::{types::tags::TagsMatches, Tag, Tags, TagsExpr};

	#[test]
	fn test_tags_macro() {
		let mut tags = Tags::new();
		tags.insert(("hello".to_owned(), "world".to_owned().into()));
		let tags_macro = tags!( "hello": "world" );
		assert_eq!(tags, tags_macro);
	}

	#[test]
	fn test_tag_macro() {
		let value: Tag = ("hello".to_owned(), "world".to_owned().into());
		let value_macro = tag!("hello": "world");
		assert_eq!(value, value_macro);
	}

	#[test]
	fn test_expr_not() {
		let expr = TagsExpr::Not(Box::new(TagsExpr::Tag(tag!("hello": "world"))));
		assert!(!expr.matches(&tags!( "hello": "world" )));
		assert!(!expr.matches(&tags!( "hello": "world", "five": "ten" )));
		assert!(expr.matches(&tags!( "five": "ten" )));
	}
}
