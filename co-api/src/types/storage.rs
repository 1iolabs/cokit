use crate::{Block, Cid};

/// Storage interface.
pub trait Storage {
	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Block;

	/// Inserts a block into storage.
	fn set(&mut self, block: Block) -> Cid;
}
