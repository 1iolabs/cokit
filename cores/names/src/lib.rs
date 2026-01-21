pub mod record;

use crate::record::RecordType;
use anyhow::anyhow;
use co_api::{
	async_api::Reducer, co, BlockStorageExt, CoId, CoMap, CoMapTransaction, CoSet, CoreBlockStorage, Did, Link,
	OptionLink, ReducerAction, TagValue,
};
use futures::{pin_mut, TryStreamExt};
use std::collections::BTreeMap;

pub type RecordId = [u8; 16];
pub type DynamicRecord = BTreeMap<String, TagValue>;
impl RecordType for DynamicRecord {
	fn record_type(&self) -> &str {
		self.get("type")
			.expect("record type property exist")
			.string()
			.expect("record type to be a string")
	}
}

#[co]
pub struct IdentityRecord {
	did: Did,
}
impl RecordType for IdentityRecord {
	fn record_type(&self) -> &str {
		"Identity"
	}
}

#[co]
pub struct CoRecord {
	co: CoId,
}
impl RecordType for CoRecord {
	fn record_type(&self) -> &str {
		"Co"
	}
}

#[co]
#[serde(tag = "type")]
#[derive(derive_more::From)]
pub enum KnownRecord {
	Identity(IdentityRecord),
	Co(CoRecord),
}
impl RecordType for KnownRecord {
	fn record_type(&self) -> &str {
		match self {
			KnownRecord::Identity(record) => record.record_type(),
			KnownRecord::Co(record) => record.record_type(),
		}
	}
}

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
impl<T: RecordType> RecordType for Record<T> {
	fn record_type(&self) -> &str {
		match self {
			Record::Known(record) => record.record_type(),
			Record::Other(record) => record.record_type(),
		}
	}
}

#[co]
pub struct IndexKey {
	pub record_type: String,
	pub name: String,
}

#[co]
pub struct IndexConfig {
	pub field: String,
	pub unique: bool,
}

#[co]
pub struct Index {
	/// The index config.
	/// Because of frequent change if `index` we store this as link.
	pub config: Link<IndexConfig>,
	pub index: CoMap<TagValue, CoSet<RecordId>>,
}

#[co]
pub struct RecordCreateAction {
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
pub struct ConfigCreateAction {
	pub key: IndexKey,
	pub config: IndexConfig,
}

#[co]
pub struct ConfigRemoveAction {
	pub key: IndexKey,
}

#[co]
#[derive(derive_more::From)]
#[non_exhaustive]
pub enum NamesAction {
	RecordCreate(RecordCreateAction),
	RecordUpdate(RecordUpdateAction),
	RecordRemove(RecordRemoveAction),
	ConfigCreate(ConfigCreateAction),
	ConfigRemove(ConfigRemoveAction),
}

#[co(state)]
pub struct Names {
	pub records: CoMap<RecordId, Link<Record>>,
	pub indexes: CoMap<Link<IndexKey>, Index>,
}
impl Reducer<NamesAction> for Names {
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<NamesAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let state = storage.get_value_or_default(&state_link).await?;
		let event = storage.get_value(&event_link).await?;
		let next_state = reduce(storage, state, event.payload).await?;
		let next_state_link = storage.set_value(&next_state).await?;
		Ok(next_state_link)
	}
}

async fn reduce(storage: &CoreBlockStorage, state: Names, action: NamesAction) -> Result<Names, anyhow::Error> {
	match action {
		NamesAction::RecordCreate(action) => reducer_record_create(storage, state, action).await,
		NamesAction::RecordUpdate(action) => reducer_record_update(storage, state, action).await,
		NamesAction::RecordRemove(action) => reducer_record_remove(storage, state, action).await,
		NamesAction::ConfigCreate(action) => reducer_config_create(storage, state, action).await,
		NamesAction::ConfigRemove(action) => reducer_config_remove(storage, state, action).await,
	}
}

pub async fn reducer_record_create(
	storage: &CoreBlockStorage,
	mut state: Names,
	action: RecordCreateAction,
) -> Result<Names, anyhow::Error> {
	// check duplicate
	let mut records = state.records.open(storage).await?;
	if records.contains_key(&action.id).await? {
		return Err(anyhow!(
			"Duplicate Record: {}",
			action.id.iter().map(|c| format!("{:02X}", c)).collect::<String>()
		));
	}

	// index
	let record: DynamicRecord = storage.get_deserialized(action.record.cid()).await?;
	let mut indexes = state.indexes.open(storage).await?;
	let indexes_stream = indexes.stream();
	pin_mut!(indexes_stream);
	while let Some((index_key_link, index)) = indexes_stream.try_next().await? {
		let index_key = storage.get_value(&index_key_link).await?;
		if index_key.record_type != record.record_type() {
			continue;
		}
		let index_config = storage.get_value(&index.config).await?;
		let mut index_transaction = index.index.open(storage).await?;
		index_record(storage, &index_key, &index_config, &mut index_transaction, &action.id, &record).await?;
		indexes
			.insert(index_key_link, Index { config: index.config, index: index_transaction.store().await? })
			.await?;
	}
	state.indexes = indexes.store().await?;

	// record
	records.insert(action.id, action.record).await?;
	state.records = records.store().await?;

	// store
	Ok(state)
}

pub async fn reducer_record_update(
	storage: &CoreBlockStorage,
	mut state: Names,
	action: RecordUpdateAction,
) -> Result<Names, anyhow::Error> {
	// get
	let mut records = state.records.open(storage).await?;
	let Some(previous_record_link) = records.get(&action.id).await? else {
		return Err(anyhow!(
			"Record not found: {}",
			action.id.iter().map(|c| format!("{:02X}", c)).collect::<String>()
		));
	};

	// index
	let mut indexes_changed = false;
	let previous_record: DynamicRecord = storage.get_deserialized(previous_record_link.cid()).await?;
	let next_record: DynamicRecord = storage.get_deserialized(action.record.cid()).await?;
	let mut indexes = state.indexes.open(storage).await?;
	let indexes_stream = indexes.stream();
	pin_mut!(indexes_stream);
	while let Some((index_key_link, index)) = indexes_stream.try_next().await? {
		let index_key = storage.get_value(&index_key_link).await?;
		let matches_previous = index_key.record_type == previous_record.record_type();
		let matches_next = index_key.record_type == next_record.record_type();
		if matches_previous || matches_next {
			let index_config = storage.get_value(&index.config).await?;

			// values
			let remove_value = if matches_previous { previous_record.get(&index_config.field) } else { None };
			let insert_value = if matches_next { next_record.get(&index_config.field) } else { None };

			// same?
			if insert_value.is_some() && insert_value == remove_value {
				continue;
			}

			// open
			let mut index_changed = false;
			let mut index_transaction = index.index.open(storage).await?;

			// remove
			if let Some(remove_value) = remove_value {
				if index_transaction.remove(remove_value.to_owned()).await?.is_some() {
					index_changed = true;
				}
			}

			// insert
			if let Some(_insert_value) = insert_value {
				index_record(storage, &index_key, &index_config, &mut index_transaction, &action.id, &next_record)
					.await?;
				index_changed = true;
			}

			// store
			if index_changed {
				indexes
					.insert(index_key_link, Index { config: index.config, index: index_transaction.store().await? })
					.await?;
				indexes_changed = true;
			}
		}
	}

	// record
	records.insert(action.id, action.record).await?;

	// result
	if indexes_changed {
		state.indexes = indexes.store().await?;
	}
	state.records = records.store().await?;
	Ok(state)
}

pub async fn reducer_record_remove(
	storage: &CoreBlockStorage,
	mut state: Names,
	action: RecordRemoveAction,
) -> Result<Names, anyhow::Error> {
	// get
	let mut records = state.records.open(storage).await?;
	let Some(record_link) = records.remove(action.id).await? else {
		return Err(anyhow!(
			"Record not found: {}",
			action.id.iter().map(|c| format!("{:02X}", c)).collect::<String>()
		));
	};
	let record: DynamicRecord = storage.get_deserialized(record_link.cid()).await?;

	// index
	let mut indexed_changed = false;
	let mut indexes = state.indexes.open(storage).await?;
	let indexes_stream = indexes.stream();
	pin_mut!(indexes_stream);
	while let Some((index_key_link, index)) = indexes_stream.try_next().await? {
		let index_key = storage.get_value(&index_key_link).await?;
		if index_key.record_type != record.record_type() {
			continue;
		}
		let index_config = storage.get_value(&index.config).await?;
		if let Some(record_value) = record.get(&index_config.field) {
			let mut index_transaction = index.index.open(storage).await?;
			if index_transaction.remove(record_value.to_owned()).await?.is_some() {
				indexes
					.insert(index_key_link, Index { config: index.config, index: index_transaction.store().await? })
					.await?;
				indexed_changed = true;
			}
		}
	}

	// result
	if indexed_changed {
		state.indexes = indexes.store().await?;
	}
	state.records = records.store().await?;
	Ok(state)
}

pub async fn reducer_config_create(
	storage: &CoreBlockStorage,
	mut state: Names,
	action: ConfigCreateAction,
) -> Result<Names, anyhow::Error> {
	let index_key_link = storage.set_value(&action.key).await?;
	let index_config_link = storage.set_value(&action.config).await?;
	let mut indexes = state.indexes.open(storage).await?;
	if indexes.contains_key(&index_key_link).await? {
		return Err(anyhow!("Duplicate Index: {:?}", action.key));
	}

	// create
	let mut index = CoMap::<TagValue, CoSet<RecordId>>::default().open(storage).await?;
	let records = state.records.open(storage).await?.stream();
	pin_mut!(records);
	while let Some((record_id, record_link)) = records.try_next().await? {
		let record: DynamicRecord = storage.get_deserialized(record_link.cid()).await?;
		if let Some(record_type) = record.get("type").and_then(TagValue::string) {
			if record_type == action.key.record_type {
				index_record(storage, &action.key, &action.config, &mut index, &record_id, &record).await?;
			}
		}
	}

	// insert
	indexes
		.insert(index_key_link, Index { config: index_config_link, index: index.store().await? })
		.await?;
	state.indexes = indexes.store().await?;
	Ok(state)
}

pub async fn reducer_config_remove(
	storage: &CoreBlockStorage,
	mut state: Names,
	action: ConfigRemoveAction,
) -> Result<Names, anyhow::Error> {
	let index_key_link = storage.set_value(&action.key).await?;
	let mut indexes = state.indexes.open(storage).await?;
	if let Some(_index) = indexes.remove(index_key_link).await? {
		state.indexes = indexes.store().await?;
	}
	Ok(state)
}

async fn index_record(
	storage: &CoreBlockStorage,
	index_key: &IndexKey,
	index_config: &IndexConfig,
	index: &mut CoMapTransaction<CoreBlockStorage, TagValue, CoSet<RecordId>>,
	record_id: &RecordId,
	record: &DynamicRecord,
) -> Result<(), anyhow::Error> {
	if let Some(record_value) = record.get(&index_config.field) {
		let mut values = index.get(record_value).await?.unwrap_or_default();
		if index_config.unique && !values.is_empty() {
			return Err(anyhow!(
				"Duplicate record {} for index {:?}",
				record_id.iter().map(|c| format!("{:02X}", c)).collect::<String>(),
				index_key
			));
		}
		values.insert(storage, *record_id).await?;
		index.insert(record_value.clone(), values).await?;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::{IdentityRecord, KnownRecord, Record};
	use co_api::{from_json, to_json_string};

	#[test]
	fn test_serialize_known_record() {
		let identity = IdentityRecord { did: "did:local:test".to_owned() };
		let record: Record = identity.into();
		let json = to_json_string(&record).unwrap();
		assert_eq!(json, r#"{"did":"did:local:test","type":"Identity"}"#);
	}

	#[test]
	fn test_deserialize_known_record() {
		let json = r#"{"did":"did:local:test","type":"Identity"}"#;
		let record: Record = from_json(json.as_bytes()).unwrap();
		assert_eq!(record, Record::Known(KnownRecord::Identity(IdentityRecord { did: "did:local:test".to_owned() })));
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
