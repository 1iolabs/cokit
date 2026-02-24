// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{create_runtime, runtimes::Runtime};
use anyhow::anyhow;
use cid::Cid;
use co_primitives::{unixfs_cat_buffer, AnyBlockStorage, KnownMultiCodec, MultiCodec};
use co_storage::StorageError;
use std::fmt::Debug;

pub struct RuntimeInstance {
	core: Cid,
	runtime: Box<dyn Runtime + Send>,
}
impl RuntimeInstance {
	/// Create a new runtime element which can be used immediately or inserted to the pool.
	pub async fn create<S>(storage: &S, core: &Cid) -> Result<Self, StorageError>
	where
		S: AnyBlockStorage,
	{
		// load
		let (native, bytes) = read_core(storage, core).await?;

		// result
		Ok(RuntimeInstance { core: *core, runtime: create_runtime(native, bytes) })
	}

	/// Create a new runtime element which can be used immediately or inserted to the pool.
	pub async fn create_native(core: &Cid, bytes: &[u8]) -> Result<Self, StorageError> {
		Ok(RuntimeInstance { core: *core, runtime: create_runtime(true, bytes.to_vec()) })
	}

	pub fn cid(&self) -> &Cid {
		&self.core
	}

	/// The runtime.
	pub fn runtime_mut(&mut self) -> &mut dyn Runtime {
		self.runtime.as_mut()
	}
}
impl Debug for RuntimeInstance {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RuntimeInstance").field("core", &self.core).finish()
	}
}

async fn read_core(storage: &impl AnyBlockStorage, cid: &Cid) -> Result<(bool, Vec<u8>), StorageError> {
	Ok(match MultiCodec::from(cid) {
		// dag-pb (unixfs)
		MultiCodec::Known(KnownMultiCodec::DagPb) => (false, unixfs_cat_buffer(storage, cid).await?),
		// raw
		MultiCodec::Known(KnownMultiCodec::Raw) => (false, storage.get(cid).await?.into_inner().1),
		_ => return Err(StorageError::InvalidArgument(anyhow!("Invalid codec"))),
	})
}
