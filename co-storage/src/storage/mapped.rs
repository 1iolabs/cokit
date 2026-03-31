// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{BlockStat, BlockStorage, BlockStorageContentMapping, StorageError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, CloneWithBlockStorageSettings, MappedCid, MultiCodec};
use std::collections::BTreeSet;

/// Maps certain CID codecs to mapped CIDs using BlockStorageContentMapping.
#[derive(Debug, Clone)]
pub struct MappedBlockStorage<S> {
	storage: S,
	codecs: BTreeSet<MultiCodec>,
}
impl<S> MappedBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + 'static,
{
	pub fn new(storage: S, codecs: BTreeSet<MultiCodec>) -> Self {
		Self { codecs, storage }
	}

	pub async fn to_mapped(&self, cid: &Cid) -> Cid {
		let codec = cid.into();
		if self.codecs.contains(&codec) {
			match self.storage.to_mapped(cid).await {
				Some(mapped) => mapped,
				None => *cid,
			}
		} else {
			*cid
		}
	}
}
#[async_trait]
impl<S> BlockStorage for MappedBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + 'static,
{
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		Ok(self.storage.get(&self.to_mapped(cid).await).await?)
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (guaranteed to be the same as the supplied).
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		Ok(self.storage.set(block).await?)
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		Ok(self.storage.remove(&self.to_mapped(cid).await).await?)
	}

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		Ok(self.storage.stat(&self.to_mapped(cid).await).await?)
	}

	fn max_block_size(&self) -> usize {
		self.storage.max_block_size()
	}
}
impl<S> CloneWithBlockStorageSettings for MappedBlockStorage<S>
where
	S: CloneWithBlockStorageSettings,
{
	fn clone_with_settings(&self, settings: co_primitives::BlockStorageCloneSettings) -> Self {
		MappedBlockStorage { storage: self.storage.clone_with_settings(settings), codecs: self.codecs.clone() }
	}
}
#[async_trait]
impl<S> BlockStorageContentMapping for MappedBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + 'static,
{
	async fn is_content_mapped(&self) -> bool {
		self.storage.is_content_mapped().await
	}

	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.storage.to_plain(mapped).await
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.storage.to_mapped(plain).await
	}

	async fn insert_mappings(&self, mappings: BTreeSet<MappedCid>) {
		self.storage.insert_mappings(mappings).await
	}
}
