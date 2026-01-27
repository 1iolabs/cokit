use crate::{
	library::permissions::{check_access_full, check_access_owner},
	record::name::reduce_name_record,
	transaction::NamesTransaction,
	Config, DynamicRecord, Index, IndexConfig, IndexInsertAction, IndexKey, IndexRemoveAction, NamesAction, Record,
	RecordId, RecordInsertAction, RecordRemoveAction, RecordType, RecordTypeLimit, RecordUpdateAction,
};
use anyhow::anyhow;
use co_api::{
	async_api::Reducer, co, BlockStorageExt, CoMap, CoMapTransaction, CoSet, CoreBlockStorage, Did, Link, OptionLink,
	ReducerAction, TagValue,
};
use futures::{pin_mut, TryStreamExt};

#[co(state)]
pub struct Names {
	#[serde(rename = "c")]
	pub config: OptionLink<Config>,
	#[serde(rename = "r")]
	pub records: CoMap<RecordId, Link<Record>>,
	#[serde(rename = "i")]
	pub indexes: CoMap<Link<IndexKey>, Index>,
}
impl Reducer<NamesAction> for Names {
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<NamesAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let mut state = storage.get_value_or_default(&state_link).await?;
		let action = storage.get_value(&event_link).await?;
		let mut transaction = NamesTransaction::open(storage.clone(), &state).await?;
		reduce(&mut transaction, action.from, action.payload).await?;
		match (state_link.link(), transaction.store(&mut state).await?) {
			(None, _) | (_, true) => Ok(storage.set_value(&state).await?),
			(Some(previous_state_link), false) => Ok(previous_state_link),
		}
	}
}

async fn reduce(state: &mut NamesTransaction, from: Did, action: NamesAction) -> Result<(), anyhow::Error> {
	match action {
		NamesAction::RecordInsert(action) => reducer_record_insert(state, from, action).await,
		NamesAction::RecordUpdate(action) => reducer_record_update(state, from, action).await,
		NamesAction::RecordRemove(action) => reducer_record_remove(state, from, action).await,
		NamesAction::IndexInsert(action) => reducer_index_insert(state, from, action).await,
		NamesAction::IndexRemove(action) => reducer_index_remove(state, from, action).await,
		NamesAction::Name(action) => reduce_name_record(state, from, action).await,
	}
}

pub async fn reducer_record_insert(
	state: &mut NamesTransaction,
	from: Did,
	action: RecordInsertAction,
) -> Result<(), anyhow::Error> {
	// permissions
	let record: DynamicRecord = state.storage().get_deserialized(action.record.cid()).await?;
	if let Some(record_config) = state.config().types.get(record.record_type()).cloned() {
		if let Some(creator) = &record_config.creator {
			// creator
			if creator != &from {
				return Err(anyhow!("Permission denied"));
			}
		}

		// limits
		if !match &record_config.limit {
			RecordTypeLimit::None => true,
			RecordTypeLimit::ByIdentity(max) => {
				let count = state.index_lookup_count(record.record_type(), "owner", from.clone()).await?;
				(*max as i32) - count > 0
			},
			RecordTypeLimit::ByRecord(max, by_record_type) => {
				let count = state.index_lookup_count(record.record_type(), "owner", from.clone()).await?;
				let by_record_count = state.index_lookup_count(by_record_type, "owner", from.clone()).await?;
				by_record_count * (*max as i32) - count > 0
			},
		} {}
	} else {
		return Err(anyhow!("Permission denied"));
	}

	// check duplicate
	if state.records().await?.contains_key(&action.id).await? {
		return Err(anyhow!(
			"Duplicate Record: {}",
			action.id.iter().map(|c| format!("{:02X}", c)).collect::<String>()
		));
	}

	// index
	let record: DynamicRecord = state.storage().get_deserialized(action.record.cid()).await?;
	let indexes_stream = state.indexes().await?.stream();
	pin_mut!(indexes_stream);
	while let Some((index_key_link, index)) = indexes_stream.try_next().await? {
		let index_key = state.storage().get_value(&index_key_link).await?;
		if index_key.record_type != record.record_type() {
			continue;
		}
		let index_config = state.storage().get_value(&index.config).await?;
		let mut index_transaction = index.index.open(state.storage()).await?;
		index_record(state.storage(), &index_key, &index_config, &mut index_transaction, &action.id, &record).await?;
		state
			.indexes_mut()
			.await?
			.insert(index_key_link, Index { config: index.config, index: index_transaction.store().await? })
			.await?;
	}

	// record
	state.records_mut().await?.insert(action.id, action.record).await?;

	// result
	Ok(())
}

pub async fn reducer_record_update(
	state: &mut NamesTransaction,
	from: Did,
	action: RecordUpdateAction,
) -> Result<(), anyhow::Error> {
	// access
	check_access_full(state, &from, &action.id).await?;

	// get
	let previous_record = state.dynamic_record(&action.id).await?;

	// index
	let next_record: DynamicRecord = state.storage().get_deserialized(action.record.cid()).await?;
	let indexes_stream = state.indexes().await?.stream();
	pin_mut!(indexes_stream);
	while let Some((index_key_link, index)) = indexes_stream.try_next().await? {
		let index_key = state.storage().get_value(&index_key_link).await?;
		let matches_previous = index_key.record_type == previous_record.record_type();
		let matches_next = index_key.record_type == next_record.record_type();
		if matches_previous || matches_next {
			let index_config = state.storage().get_value(&index.config).await?;

			// values
			let remove_value = if matches_previous { previous_record.get(&index_config.field) } else { None };
			let insert_value = if matches_next { next_record.get(&index_config.field) } else { None };

			// same?
			if insert_value.is_some() && insert_value == remove_value {
				continue;
			}

			// open
			let mut index_changed = false;
			let mut index_transaction = index.index.open(state.storage()).await?;

			// remove
			if let Some(remove_value) = remove_value {
				if index_transaction.remove(remove_value.to_owned()).await?.is_some() {
					index_changed = true;
				}
			}

			// insert
			if let Some(_insert_value) = insert_value {
				index_record(
					state.storage(),
					&index_key,
					&index_config,
					&mut index_transaction,
					&action.id,
					&next_record,
				)
				.await?;
				index_changed = true;
			}

			// store
			if index_changed {
				state
					.indexes_mut()
					.await?
					.insert(index_key_link, Index { config: index.config, index: index_transaction.store().await? })
					.await?;
			}
		}
	}

	// record
	state.records_mut().await?.insert(action.id, action.record).await?;

	// result
	Ok(())
}

pub async fn reducer_record_remove(
	state: &mut NamesTransaction,
	from: Did,
	action: RecordRemoveAction,
) -> Result<(), anyhow::Error> {
	// access
	check_access_owner(state, &from, &action.id).await?;

	// get
	let record: DynamicRecord = state.dynamic_record(&action.id).await?;

	// index
	let indexes_stream = state.indexes().await?.stream();
	pin_mut!(indexes_stream);
	while let Some((index_key_link, index)) = indexes_stream.try_next().await? {
		let index_key = state.storage().get_value(&index_key_link).await?;
		if index_key.record_type != record.record_type() {
			continue;
		}
		let index_config = state.storage().get_value(&index.config).await?;
		if let Some(record_value) = record.get(&index_config.field) {
			let mut index_transaction = index.index.open(state.storage()).await?;
			if index_transaction.remove(record_value.to_owned()).await?.is_some() {
				state
					.indexes_mut()
					.await?
					.insert(index_key_link, Index { config: index.config, index: index_transaction.store().await? })
					.await?;
			}
		}
	}

	// result
	Ok(())
}

pub async fn reducer_index_insert(
	state: &mut NamesTransaction,
	_from: Did,
	action: IndexInsertAction,
) -> Result<(), anyhow::Error> {
	let index_key_link = state.storage().set_value(&action.key).await?;
	let index_config_link = state.storage().set_value(&action.config).await?;
	if state.indexes().await?.contains_key(&index_key_link).await? {
		return Err(anyhow!("Duplicate Index: {:?}", action.key));
	}

	// create
	let mut index = CoMap::<TagValue, CoSet<RecordId>>::default().open(state.storage()).await?;
	let records = state.records().await?.stream();
	pin_mut!(records);
	while let Some((record_id, record_link)) = records.try_next().await? {
		let record: DynamicRecord = state.storage().get_deserialized(record_link.cid()).await?;
		if let Some(record_type) = record.get("type").and_then(TagValue::string) {
			if record_type == action.key.record_type {
				index_record(state.storage(), &action.key, &action.config, &mut index, &record_id, &record).await?;
			}
		}
	}

	// insert
	state
		.indexes_mut()
		.await?
		.insert(index_key_link, Index { config: index_config_link, index: index.store().await? })
		.await?;
	Ok(())
}

pub async fn reducer_index_remove(
	state: &mut NamesTransaction,
	_from: Did,
	action: IndexRemoveAction,
) -> Result<(), anyhow::Error> {
	let index_key_link = state.storage().set_value(&action.key).await?;
	state.indexes_mut().await?.remove(index_key_link).await?;
	Ok(())
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
