use ipld_core::ipld::Ipld;

pub trait ObjectAPI {
	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Result<Ipld, StorageError>;

	/// Inserts a block into storage.
	fn set(&mut self, object: Ipld) -> Result<Cid, StorageError>;
}
