// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::permissions::check_access, record::KnownRecord, transaction::NamesTransaction, Record, RecordId,
	RecordType, NAME_RECORD_TYPE,
};
use co_api::{co, tags, BlockStorageExt, CoId, Did, TagValue, Tags};
use std::borrow::Cow;

/// Name Record.
#[co]
pub struct NameRecord {
	/// Optional. Controllers of this record.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub controller: Vec<Did>,

	/// The owner of this record.
	pub owner: Did,

	/// The name.
	pub name: String,

	/// The parent of this name.
	/// This is used to form hierarchial names.
	/// If no parent is specified this is a top level record.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub parent: Option<RecordId>,

	/// Optional. Endpoints.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub endpoint: Vec<Endpoint>,

	/// Optional. Indicates that child names are managed in a names core in a different CO.
	/// If specified this name can nur used for children NameRecord's inside this core.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub children: Option<CoId>,
}
impl RecordType for NameRecord {
	fn record_type(&self) -> &str {
		"Name"
	}

	fn controller(&self) -> Cow<'_, Vec<Did>> {
		Cow::Borrowed(&self.controller)
	}

	fn owner(&self) -> Option<&Did> {
		Some(&self.owner)
	}
}

#[co]
pub enum EndpointScheme {
	Multiaddr,
	Uri,
}

pub type EndpointId = [u8; 16];

#[co]
pub struct Endpoint {
	pub id: EndpointId,
	pub scheme: EndpointScheme,
	pub value: TagValue,
}

#[co]
#[derive(derive_more::From)]
pub enum NameRecordAction {
	EndpointInsert(EndpointInsertAction),
	EndpointRemove(EndpointRemoveAction),
}
impl NameRecordAction {
	pub fn record(&self) -> RecordId {
		match self {
			NameRecordAction::EndpointInsert(action) => action.record,
			NameRecordAction::EndpointRemove(action) => action.record,
		}
	}

	pub fn scope(&self) -> Tags {
		match self {
			NameRecordAction::EndpointInsert(_) => tags!("action": "name-endpoint-insert"),
			NameRecordAction::EndpointRemove(_) => tags!("action": "name-endpoint-remove"),
		}
	}
}

#[co]
pub struct EndpointInsertAction {
	pub record: RecordId,

	/// The endpoint to insert.
	pub endpoint: Endpoint,

	/// Insert the endpoint before specified endpoint.
	/// If not set (or not found) the record will be created at last position.
	pub before: Option<EndpointId>,
}

#[co]
pub struct EndpointRemoveAction {
	pub record: RecordId,

	/// The endpoint to remove.
	pub id: EndpointId,
}

pub async fn reduce_name_record(
	state: &mut NamesTransaction,
	from: Did,
	action: NameRecordAction,
) -> Result<(), anyhow::Error> {
	match action {
		NameRecordAction::EndpointInsert(action) => reduce_endpoint_insert(state, from, action).await,
		NameRecordAction::EndpointRemove(action) => reduce_endpoint_remove(state, from, action).await,
	}
}

pub async fn reduce_endpoint_insert(
	state: &mut NamesTransaction,
	from: Did,
	action: EndpointInsertAction,
) -> Result<(), anyhow::Error> {
	// access
	check_access(state, &from, &action.record, tags!("type": NAME_RECORD_TYPE, "action": "EndpointInsert")).await?;

	// apply
	if let Record::Known(KnownRecord::Name(mut name_record)) = state.record(&action.record).await? {
		// insert
		let index = action
			.before
			.and_then(|before| find_endpoint_by_id(&name_record.endpoint, &before))
			.map(|(index, _)| index)
			.unwrap_or(name_record.endpoint.len());
		name_record.endpoint.insert(index, action.endpoint);

		// store
		let record: Record = name_record.into();
		let record_link = state.storage().set_value(&record).await?;
		state.records_mut().await?.insert(action.record, record_link).await?;
	}

	// result
	Ok(())
}

pub async fn reduce_endpoint_remove(
	state: &mut NamesTransaction,
	from: Did,
	action: EndpointRemoveAction,
) -> Result<(), anyhow::Error> {
	// access
	check_access(state, &from, &action.record, tags!("type": NAME_RECORD_TYPE, "action": "EndpointRemove")).await?;

	// apply
	if let Record::Known(KnownRecord::Name(mut name_record)) = state.record(&action.record).await? {
		// remove
		if let Some((index, _)) = find_endpoint_by_id(&name_record.endpoint, &action.id) {
			// remove
			name_record.endpoint.remove(index);

			// store
			let record: Record = name_record.into();
			let record_link = state.storage().set_value(&record).await?;
			state.records_mut().await?.insert(action.record, record_link).await?;
		}
	}

	// result
	Ok(())
}

fn find_endpoint_by_id<'a>(endpoints: &'a [Endpoint], id: &EndpointId) -> Option<(usize, &'a Endpoint)> {
	endpoints.iter().enumerate().find(|(_index, item)| &item.id == id)
}
