use crate::{create_runtime, runtimes::Runtime, CoreDescriptor};
use anyhow::anyhow;
use cid::Cid;
use co_primitives::{AnyBlockStorage, BlockStorageExt, KnownMultiCodec, MultiCodec};
use co_storage::{unixfs_cat_buffer, StorageError};
use std::fmt::Debug;

pub struct RuntimeInstance {
	cid: Cid,
	runtime: Box<dyn Runtime + Send>,
}
impl RuntimeInstance {
	/// Create a new runtime element which can be used immediately or inserted to the pool.
	pub async fn create<S>(storage: &S, cid: &Cid) -> Result<Self, StorageError>
	where
		S: AnyBlockStorage,
	{
		// load
		let (native, bytes) = read_core(storage, cid).await?;

		// result
		Ok(RuntimeInstance { cid: *cid, runtime: create_runtime(native, bytes) })
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

async fn read_core(storage: &impl AnyBlockStorage, cid: &Cid) -> Result<(bool, Vec<u8>), StorageError> {
	Ok(match MultiCodec::from(cid) {
		// dag-pb (unixfs)
		MultiCodec::Known(KnownMultiCodec::DagPb) => (false, unixfs_cat_buffer(storage, cid).await?),
		// raw
		MultiCodec::Known(KnownMultiCodec::Raw) => (false, storage.get(cid).await?.into_inner().1),
		MultiCodec::Known(KnownMultiCodec::DagCbor) => {
			let descriptor: CoreDescriptor = storage.get_deserialized(cid).await?;
			let host = target_lexicon::HOST.to_string();
			if let Some(arch) = descriptor.native.get(&host) {
				(true, Box::pin(read_core(storage, arch)).await?.1)
			} else {
				Box::pin(read_core(storage, &descriptor.wasm)).await?
			}
		},
		_ => return Err(StorageError::InvalidArgument(anyhow!("Invalid codec"))),
	})
}
