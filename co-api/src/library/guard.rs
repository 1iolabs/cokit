// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::{
		data::{read_input_sync, write_output_sync},
		wasm_storage::WasmStorage,
	},
	Guard,
};
use co_primitives::{CoreBlockStorage, GuardInput, GuardOutput, RawCid, Tags};
use futures::{executor::LocalPool, future::LocalBoxFuture, task::LocalSpawnExt, FutureExt};
use std::sync::Arc;

pub fn guard<R>(input: &RawCid, output: &mut RawCid)
where
	R: Guard + 'static,
{
	let mut storage = WasmStorage::new();
	let block_storage = CoreBlockStorage::new(storage.clone(), false);

	// input
	let guard_input: GuardInput = read_input_sync(&storage, input);

	// execute
	let guard_output = GuardRef::new::<R>().execute_blocking(guard_input, block_storage);

	// output
	write_output_sync(&mut storage, &guard_output, output);
}

#[allow(clippy::type_complexity)]
pub struct GuardRef(
	Arc<dyn Fn(GuardInput, CoreBlockStorage) -> LocalBoxFuture<'static, GuardOutput> + Sync + Send + 'static>,
);
impl GuardRef {
	pub fn new<R>() -> Self
	where
		R: Guard + 'static,
	{
		Self(Arc::new(|input, storage| {
			async move {
				match R::verify(&storage, input.guard, input.state, input.heads, input.next_head).await {
					Ok(valid) => GuardOutput { result: valid, error: None, tags: Tags::default() },
					Err(err) => GuardOutput { result: false, error: Some(err.to_string()), tags: Tags::default() },
				}
			}
			.boxed_local()
		}))
	}

	pub fn execute_blocking(&self, input: GuardInput, storage: CoreBlockStorage) -> GuardOutput {
		let mut pool = LocalPool::new();
		let handle = pool
			.spawner()
			.spawn_local_with_handle({
				let execute = self.0.clone();
				async move { execute(input, storage).await }
			})
			.expect("future to execute");
		pool.run_until(handle)
	}

	pub async fn execute_async(&self, input: GuardInput, storage: CoreBlockStorage) -> GuardOutput {
		(self.0)(input, storage).await
	}
}
impl Clone for GuardRef {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
