use crate::NameRecord;
use co_api::{co, CoId, Did, TagValue, TagsExpr};
use std::{borrow::Cow, collections::BTreeMap};

pub mod name;

pub const NAME_RECORD_TYPE: &str = "Name";
pub const DELEGATE_RECORD_TYPE: &str = "Delegate";
pub const CO_RECORD_TYPE: &str = "Co";

/// Record.
#[co]
#[serde(untagged)]
pub enum Record<T = DynamicRecord>
where
	T: RecordType,
{
	Known(KnownRecord),
	Other(T),
}
impl<T, R> From<R> for Record<T>
where
	T: RecordType,
	R: Into<KnownRecord>,
{
	fn from(value: R) -> Self {
		Self::Known(value.into())
	}
}
impl<T: RecordType> Record<T> {
	fn record(&self) -> &dyn RecordType {
		match self {
			Record::Known(record) => record,
			Record::Other(record) => record,
		}
	}
}
impl<T: RecordType> RecordType for Record<T> {
	fn record_type(&self) -> &str {
		self.record().record_type()
	}

	fn controller(&self) -> Cow<'_, Vec<Did>> {
		self.record().controller()
	}

	fn owner(&self) -> Option<&Did> {
		self.record().owner()
	}
}

/// Known Record Types.
#[co]
#[serde(tag = "type")]
#[derive(derive_more::From)]
pub enum KnownRecord {
	Name(NameRecord),
	Co(CoRecord),
	Delegate(DelegateRecord),
	Identity(IdentityRecord),
}
impl KnownRecord {
	fn record(&self) -> &dyn RecordType {
		match self {
			KnownRecord::Identity(record) => record,
			KnownRecord::Co(record) => record,
			KnownRecord::Name(record) => record,
			KnownRecord::Delegate(record) => record,
		}
	}
}
impl RecordType for KnownRecord {
	fn record_type(&self) -> &str {
		self.record().record_type()
	}

	fn controller(&self) -> Cow<'_, Vec<Did>> {
		self.record().controller()
	}

	fn owner(&self) -> Option<&Did> {
		self.record().owner()
	}
}

/// Record type.
pub trait RecordType {
	fn record_type(&self) -> &str;
	fn controller(&self) -> Cow<'_, Vec<Did>>;
	fn owner(&self) -> Option<&Did>;
}

/// Record ID.
/// Usually a UUID in binary form.
pub type RecordId = [u8; 16];

/// Dynamic Record.
pub type DynamicRecord = BTreeMap<String, TagValue>;
impl RecordType for DynamicRecord {
	fn record_type(&self) -> &str {
		self.get("type")
			.expect("record type property exist")
			.string()
			.expect("record type to be a string")
	}

	fn controller(&self) -> Cow<'_, Vec<Did>> {
		Cow::Owned(match self.get("controller") {
			Some(TagValue::String(value)) => vec![value.clone()],
			Some(TagValue::List(value)) => value
				.iter()
				.filter_map(|value| if let TagValue::String(str) = value { Some(str.clone()) } else { None })
				.collect(),
			_ => Default::default(),
		})
	}

	fn owner(&self) -> Option<&Did> {
		match self.get("owner") {
			Some(TagValue::String(value)) => Some(value),
			_ => None,
		}
	}
}

/// Identity Record.
#[co]
pub struct IdentityRecord {
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub controller: Vec<Did>,
	pub id: Did,
}
impl RecordType for IdentityRecord {
	fn record_type(&self) -> &str {
		"Identity"
	}

	fn controller(&self) -> Cow<'_, Vec<Did>> {
		Cow::Borrowed(&self.controller)
	}

	fn owner(&self) -> Option<&Did> {
		Some(&self.id)
	}
}

/// Co Record.
#[co]
pub struct CoRecord {
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub controller: Vec<Did>,
	pub owner: Did,
	pub co: CoId,
}
impl RecordType for CoRecord {
	fn record_type(&self) -> &str {
		"Co"
	}

	fn controller(&self) -> Cow<'_, Vec<Did>> {
		Cow::Borrowed(&self.controller)
	}

	fn owner(&self) -> Option<&Did> {
		Some(&self.owner)
	}
}

/// Delegate Record.
#[co]
pub struct DelegateRecord {
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub controller: Vec<Did>,

	pub owner: Did,
	pub to: Did,

	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub record: Vec<RecordId>,

	/// The delegated scope. None means full access.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub scope: Option<TagsExpr>,
}
impl RecordType for DelegateRecord {
	fn record_type(&self) -> &str {
		DELEGATE_RECORD_TYPE
	}

	fn controller(&self) -> Cow<'_, Vec<Did>> {
		Cow::Borrowed(&self.controller)
	}

	fn owner(&self) -> Option<&Did> {
		Some(&self.owner)
	}
}

#[cfg(test)]
mod tests {
	use super::{IdentityRecord, KnownRecord, Record};
	use co_api::{from_json, to_json_string};

	#[test]
	fn test_serialize_known_record() {
		let identity = IdentityRecord { id: "did:local:test".to_owned(), controller: Default::default() };
		let record: Record = identity.into();
		let json = to_json_string(&record).unwrap();
		assert_eq!(json, r#"{"id":"did:local:test","type":"Identity"}"#);
	}

	#[test]
	fn test_deserialize_known_record() {
		let json = r#"{"id":"did:local:test","type":"Identity"}"#;
		let record: Record = from_json(json.as_bytes()).unwrap();
		assert_eq!(
			record,
			Record::Known(KnownRecord::Identity(IdentityRecord {
				id: "did:local:test".to_owned(),
				controller: Default::default()
			}))
		);
	}

	#[test]
	fn test_deserialize_dynamic_record() {
		let json = r#"{"did":"did:local:test","type":"Dynamic"}"#;
		let record: Record = from_json(json.as_bytes()).unwrap();
		assert_eq!(
			record,
			Record::Other(
				[
					("did".to_owned(), "did:local:test".to_owned().into()),
					("type".to_owned(), "Dynamic".to_owned().into()),
				]
				.into_iter()
				.collect()
			)
		);
	}
}
