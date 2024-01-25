use crate::{create_runtime, runtimes::Runtime};
use anyhow::anyhow;
use co_storage::{unixfs_cat_buffer, BlockStorage, StorageError};
use libipld::Cid;
use std::fmt::Debug;

pub struct RuntimeInstance {
	cid: Cid,
	runtime: Box<dyn Runtime + Send>,
}
impl RuntimeInstance {
	/// Create a new runtime element which can be used immediately or inserted to the pool.
	pub async fn create<S>(storage: &S, cid: &Cid) -> Result<Self, StorageError>
	where
		S: BlockStorage + Send,
	{
		// load
		let wasm_bytes: Vec<u8> = match cid.codec() {
			// dag-pb (unixfs)
			0x70 => unixfs_cat_buffer(storage, cid).await?,
			// raw
			0x55 => storage.get(cid).await?.into_inner().1,
			_ => return Err(StorageError::InvalidArgument(anyhow!("Invalid codec"))),
		};

		// result
		Ok(RuntimeInstance { cid: cid.clone(), runtime: create_runtime(wasm_bytes) })
	}

	pub fn cid(&self) -> &Cid {
		&self.cid
	}

	/// The runtime.
	pub fn runtime_mut(&mut self) -> &mut dyn Runtime {
		self.runtime.as_mut()
	}
}
impl Debug for RuntimeInstance {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RuntimeInstance").field("cid", &self.cid).finish()
	}
}
