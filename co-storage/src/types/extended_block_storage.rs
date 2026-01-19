use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorage, MappedCid, StorageError};
use std::collections::BTreeMap;

#[async_trait]
pub trait ExtendedBlockStorage: BlockStorage {
	/// Inserts a block into storage.
	async fn set_extended(&self, block: ExtendedBlock) -> Result<Cid, StorageError>;

	/// Test if a Cid exists.
	///
	/// Note: This is an local operation and will not fetch from network.
	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError>;

	/// Clear the storage by removing all entries.
	async fn clear(&self) -> Result<(), StorageError>;
}

#[derive(Debug, Clone)]
pub struct ExtendedBlock {
	pub block: Block,
	pub options: ExtendedBlockOptions,
}
impl ExtendedBlock {
	pub fn new(block: Block) -> Self {
		Self { block, options: Default::default() }
	}

	pub fn with_options(mut self, options: ExtendedBlockOptions) -> Self {
		self.options = options;
		self
	}

	pub fn with_references(mut self, references: impl IntoIterator<Item = MappedCid>) -> Self {
		self.options = self.options.with_references(references);
		self
	}
}
impl From<Block> for ExtendedBlock {
	fn from(block: Block) -> Self {
		Self::new(block)
	}
}
impl From<(Block, ExtendedBlockOptions)> for ExtendedBlock {
	fn from(value: (Block, ExtendedBlockOptions)) -> Self {
		Self { block: value.0, options: value.1 }
	}
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtendedBlockOptions {
	// Extra references.
	pub references: Option<BTreeMap<Cid, Cid>>,
}
impl ExtendedBlockOptions {
	pub fn with_references(mut self, references: impl IntoIterator<Item = MappedCid>) -> Self {
		self.references = Some(
			references
				.into_iter()
				.map(|MappedCid(internal, external)| (internal, external))
				.collect(),
		);
		self
	}
}
