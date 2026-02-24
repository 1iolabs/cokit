// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_primitives::{Block, StorageError, StoreParams};

/// Storage interface.
pub trait Storage {
	type StoreParams: StoreParams;

	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Result<Block, StorageError>;

	/// Inserts a block into storage.
	fn set(&mut self, block: Block) -> Result<Cid, StorageError>;

	/// Remove a block from storage.
	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError>;
}
