// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
