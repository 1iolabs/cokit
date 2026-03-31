// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::{
		data::{read_input_sync, write_output_sync},
		wasm_storage::WasmStorage,
	},
	Reducer,
};
use co_primitives::{CoreBlockStorage, RawCid, ReducerInput, ReducerOutput, Tags};
use futures::{executor::LocalPool, future::LocalBoxFuture, task::LocalSpawnExt, FutureExt};
use serde::de::DeserializeOwned;
use std::sync::Arc;

#[allow(clippy::type_complexity)]
pub struct ReducerRef(
	Arc<dyn Fn(ReducerInput, CoreBlockStorage) -> LocalBoxFuture<'static, ReducerOutput> + Sync + Send + 'static>,
);
impl ReducerRef {
	pub fn new<R, A>() -> Self
	where
		R: Reducer<A> + 'static,
		A: Clone + DeserializeOwned + 'static,
	{
		Self(Arc::new(|input, storage| {
			async move {
				let state = input.state;
				match R::reduce(state.into(), input.action.into(), &storage).await {
					Ok(link) => ReducerOutput { state: Some(link.into()), error: None, tags: Tags::default() },
					Err(err) => ReducerOutput { state, error: Some(err.to_string()), tags: Tags::default() },
				}
			}
			.boxed_local()
		}))
	}

	pub fn execute_blocking(&self, input: ReducerInput, storage: CoreBlockStorage) -> ReducerOutput {
		let closure = self.0.clone();
		let mut pool = LocalPool::new();
		let handle = pool
			.spawner()
			.spawn_local_with_handle(async move { closure(input, storage).await })
			.expect("future to execute");
		pool.run_until(handle)
	}

	pub async fn execute_async(&self, input: ReducerInput, storage: CoreBlockStorage) -> ReducerOutput {
		(self.0)(input, storage).await
	}
}
impl Clone for ReducerRef {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

pub fn reduce<R, A>(input: &RawCid, output: &mut RawCid)
where
	R: Reducer<A> + 'static,
	A: Clone + DeserializeOwned + 'static,
{
	let mut storage = WasmStorage::new();
	let block_storage = CoreBlockStorage::new(storage.clone(), false);

	// input
	let reducer_input: ReducerInput = read_input_sync(&storage, input);

	// reduce
	let reducer_output = ReducerRef::new::<R, A>().execute_blocking(reducer_input, block_storage);

	// output
	write_output_sync(&mut storage, &reducer_output, output);
}
