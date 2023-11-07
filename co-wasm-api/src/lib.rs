use libipld::{Block, Cid, DefaultParams};

/// Storage interface.
pub trait Storage {
	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Block<DefaultParams>;

	/// Inserts a block into storage.
	fn set(&mut self, block: Block<DefaultParams>);
}
