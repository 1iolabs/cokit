use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorage, StorageError};
use std::collections::BTreeMap;

#[async_trait]
pub trait ExtendedBlockStorage: BlockStorage {
	/// Inserts a block into storage.
	async fn set_extended(&self, block: ExtendedBlock<Self::StoreParams>) -> Result<Cid, StorageError>;

	/// Test if a Cid exists.
	///
	/// Note: This is an local operation and will not fetch from network.
	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError>;

	/// Clear the storage by removing all entries.
	async fn clear(&self) -> Result<(), StorageError>;
}

#[derive(Debug, Clone)]
pub struct ExtendedBlock<P> {
	pub block: Block<P>,
	pub options: ExtendedBlockOptions,
}
impl<P> ExtendedBlock<P> {
	pub fn new(block: Block<P>) -> Self {
		Self { block, options: Default::default() }
	}

	pub fn with_options(mut self, options: ExtendedBlockOptions) -> Self {
		self.options = options;
		self
	}

	pub fn with_references(mut self, references: impl IntoIterator<Item = (Cid, Cid)>) -> Self {
		self.options = self.options.with_references(references);
		self
	}
}
impl<P> From<Block<P>> for ExtendedBlock<P> {
	fn from(block: Block<P>) -> Self {
		Self::new(block)
	}
}
impl<P> From<(Block<P>, ExtendedBlockOptions)> for ExtendedBlock<P> {
	fn from(value: (Block<P>, ExtendedBlockOptions)) -> Self {
		Self { block: value.0, options: value.1 }
	}
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtendedBlockOptions {
	// Extra references.
	pub references: Option<BTreeMap<Cid, Cid>>,
}
impl ExtendedBlockOptions {
	pub fn with_references(mut self, references: impl IntoIterator<Item = (Cid, Cid)>) -> Self {
		self.references = Some(references.into_iter().collect());
		self
	}
}
