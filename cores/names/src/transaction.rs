// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{Config, DynamicRecord, Index, IndexKey, Names, Record, RecordId};
use co_api::{
	BlockSerializer, BlockStorageExt, CoMap, CoMapTransaction, CoSet, CoreBlockStorage, LazyTransaction, Link,
	StorageError, TagValue,
};
use futures::{pin_mut, Stream, TryStreamExt};
use std::future::ready;

pub struct NamesTransaction {
	pub storage: CoreBlockStorage,
	pub config: Config,
	pub records: LazyTransaction<CoreBlockStorage, CoMap<RecordId, Link<Record>>>,
	pub indexes: LazyTransaction<CoreBlockStorage, CoMap<Link<IndexKey>, Index>>,
}
impl NamesTransaction {
	pub async fn open(storage: CoreBlockStorage, state: &Names) -> Result<Self, anyhow::Error> {
		Ok(Self {
			config: storage.get_value_or_default(&state.config).await?,
			records: state.records.open_lazy(&storage).await?,
			indexes: state.indexes.open_lazy(&storage).await?,
			storage,
		})
	}

	pub async fn store(&mut self, state: &mut Names) -> Result<bool, anyhow::Error> {
		let mut changed = false;
		if let Some(records) = self.records.opt_if_is_mut_access() {
			state.records = records.store().await?;
			changed = true;
		}
		if let Some(indexes) = self.indexes.opt_if_is_mut_access() {
			state.indexes = indexes.store().await?;
			changed = true;
		}
		Ok(changed)
	}

	pub fn storage(&self) -> &CoreBlockStorage {
		&self.storage
	}

	pub fn config(&self) -> &Config {
		&self.config
	}

	pub async fn records(
		&mut self,
	) -> Result<&CoMapTransaction<CoreBlockStorage, RecordId, Link<Record>>, StorageError> {
		self.records.get().await
		// SAFEITY: Has been just intialized.
		// Ok(self.records.opt().unwrap())
	}

	pub async fn records_mut(
		&mut self,
	) -> Result<&mut CoMapTransaction<CoreBlockStorage, RecordId, Link<Record>>, StorageError> {
		self.records.get_mut().await
	}

	pub async fn indexes(
		&mut self,
	) -> Result<&CoMapTransaction<CoreBlockStorage, Link<IndexKey>, Index>, StorageError> {
		self.indexes.get().await
	}

	pub async fn indexes_mut(
		&mut self,
	) -> Result<&mut CoMapTransaction<CoreBlockStorage, Link<IndexKey>, Index>, StorageError> {
		self.indexes.get_mut().await
	}

	pub async fn record(&mut self, id: &RecordId) -> Result<Record, anyhow::Error> {
		record(&self.storage, self.records.get().await?, id).await
	}

	pub async fn dynamic_record(&mut self, id: &RecordId) -> Result<DynamicRecord, anyhow::Error> {
		dynamic_record(&self.storage, self.records.get().await?, id).await
	}

	// pub async fn index_lookup(
	// 	&mut self,
	// 	record_type: &str,
	// 	name: &str,
	// 	value: impl Into<TagValue>,
	// ) -> Result<CoSet<RecordId>, anyhow::Error> {
	// 	index_lookup(&self.storage, self.indexes.get().await?, record_type, name, value).await
	// }

	pub async fn index_lookup_count(
		&mut self,
		record_type: &str,
		name: &str,
		value: impl Into<TagValue>,
	) -> Result<i32, anyhow::Error> {
		index_lookup_count(&self.storage, self.indexes.get().await?, record_type, name, value).await
	}

	pub async fn index_lookup_records(
		&mut self,
		record_type: impl Into<String>,
		name: impl Into<String>,
		value: impl Into<TagValue>,
	) -> Result<impl Stream<Item = Result<Record, anyhow::Error>> + 'static, anyhow::Error> {
		let record_type = record_type.into();
		let name = name.into();
		let value = value.into();
		let storage = self.storage.clone();
		let records = self.records.get().await?.clone();
		let indexes = self.indexes.get().await?.clone();
		Ok(index_lookup_records(records, indexes, storage, record_type, name, value))
	}
}

async fn record(
	storage: &CoreBlockStorage,
	records: &CoMapTransaction<CoreBlockStorage, RecordId, Link<Record>>,
	id: &RecordId,
) -> Result<Record, anyhow::Error> {
	let Some(record_link) = records.get(id).await? else {
		return Err(anyhow::anyhow!(
			"Record not found: {}",
			id.iter().map(|c| format!("{:02X}", c)).collect::<String>()
		));
	};
	Ok(storage.get_value(&record_link).await?)
}

async fn dynamic_record(
	storage: &CoreBlockStorage,
	records: &CoMapTransaction<CoreBlockStorage, RecordId, Link<Record>>,
	id: &RecordId,
) -> Result<DynamicRecord, anyhow::Error> {
	let Some(record_link) = records.get(id).await? else {
		return Err(anyhow::anyhow!(
			"Record not found: {}",
			id.iter().map(|c| format!("{:02X}", c)).collect::<String>()
		));
	};
	Ok(storage.get_deserialized(record_link.cid()).await?)
}

async fn index_lookup(
	storage: &CoreBlockStorage,
	indexes: &CoMapTransaction<CoreBlockStorage, Link<IndexKey>, Index>,
	record_type: &str,
	name: &str,
	value: impl Into<TagValue>,
) -> Result<CoSet<RecordId>, anyhow::Error> {
	let key = IndexKey { record_type: record_type.to_owned(), name: name.to_owned() };
	let key_block = BlockSerializer::new().serialize(&key)?;
	if let Some(index) = indexes.get(&key_block.cid().into()).await? {
		if let Some(records) = index.index.get(storage, &value.into()).await? {
			return Ok(records);
		}
	}
	Ok(Default::default())
}

pub async fn index_lookup_count(
	storage: &CoreBlockStorage,
	indexes: &CoMapTransaction<CoreBlockStorage, Link<IndexKey>, Index>,
	record_type: &str,
	name: &str,
	value: impl Into<TagValue>,
) -> Result<i32, anyhow::Error> {
	Ok(index_lookup(storage, indexes, record_type, name, value)
		.await?
		.into_stream(storage.clone())
		.try_fold(0, |result, _id| ready(Ok(result + 1)))
		.await?)
}

fn index_lookup_records(
	records: CoMapTransaction<CoreBlockStorage, RecordId, Link<Record>>,
	indexes: CoMapTransaction<CoreBlockStorage, Link<IndexKey>, Index>,
	storage: CoreBlockStorage,
	record_type: String,
	name: String,
	value: TagValue,
) -> impl Stream<Item = Result<Record, anyhow::Error>> + 'static {
	async_stream::try_stream! {
		let record_ids = index_lookup(&storage, &indexes, &record_type, &name, value).await?;
		let record_ids_stream = record_ids.stream(&storage);
		pin_mut!(record_ids_stream);
		while let Some(record_id) = record_ids_stream.try_next().await? {
			yield record(&storage, &records, &record_id).await?;
		}
	}
}
