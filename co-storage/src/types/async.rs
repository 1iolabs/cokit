// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

/// Async storage interface.
#[async_trait::async_trait(?Send)]
pub trait AsyncStorage {
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError>;

	/// Inserts a block into storage.
	async fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError>;

	/// Remove a block from storage.
	async fn remove(&mut self, cid: &Cid) -> Result<(), StorageError>;
}

struct AsyncStorageWrapper {
	next: Box<dyn Storage>,
}

#[async_trait::async_trait(?Send)]
impl AsyncStorage for AsyncStorageWrapper {
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		self.next.get(cid)
	}

	/// Inserts a block into storage.
	async fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError> {
		self.next.set(block)
	}

	/// Inserts a block into storage.
	async fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		self.next.remove(cid)
	}
}

/// Execute async storage using an runtime.
///
/// See: https://tokio.rs/tokio/topics/bridging
struct StorageWrapper {
	runtime: tokio::runtime::Runtime,
	next: Box<dyn AsyncStorage>,
}

impl Storage for StorageWrapper {
	fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		self.runtime.block_on(self.next.get(cid))?
	}

	fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError> {
		self.runtime.block_on(self.next.set(block))?
	}

	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		self.runtime.block_on(self.next.remove(cid))?
	}
}
