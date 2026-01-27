use crate::{IndexConfig, IndexKey, NameRecordAction, Record, RecordId};
use co_api::{co, Link};

#[co]
#[derive(derive_more::From)]
#[non_exhaustive]
pub enum NamesAction {
	#[serde(rename = "rc")]
	RecordInsert(RecordInsertAction),
	#[serde(rename = "ru")]
	RecordUpdate(RecordUpdateAction),
	#[serde(rename = "rr")]
	RecordRemove(RecordRemoveAction),
	#[serde(rename = "cc")]
	IndexInsert(IndexInsertAction),
	#[serde(rename = "cr")]
	IndexRemove(IndexRemoveAction),
	#[serde(rename = "nr")]
	Name(NameRecordAction),
}

#[co]
pub struct RecordInsertAction {
	pub id: RecordId,
	pub record: Link<Record>,
}

#[co]
pub struct RecordUpdateAction {
	pub id: RecordId,
	pub record: Link<Record>,
}

#[co]
pub struct RecordRemoveAction {
	pub id: RecordId,
}

#[co]
pub struct IndexInsertAction {
	pub key: IndexKey,
	pub config: IndexConfig,
}

#[co]
pub struct IndexRemoveAction {
	pub key: IndexKey,
}
