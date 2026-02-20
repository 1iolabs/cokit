use super::wasm_context::WasmContext;
use crate::{
	sync_api::{Context, Reducer},
	Block, Cid, ReducerAction,
};
use co_primitives::{from_cbor, to_cbor, KnownMultiCodec};
use multihash_codetable::{Code, MultihashDigest};
use serde::{de::DeserializeOwned, Serialize};

pub fn reduce<S>()
where
	S: Reducer + Default + Serialize + DeserializeOwned,
	S::Action: DeserializeOwned,
{
	let mut context = WasmContext::new();
	reduce_with_context::<S>(&mut context)
}

pub fn reduce_with_context<S>(context: &mut dyn Context)
where
	S: Reducer + Default + Serialize + DeserializeOwned,
	S::Action: DeserializeOwned,
{
	// state
	let cid = context.state();
	let state = match cid {
		None => S::default(),
		Some(cid) => {
			let block = context.storage().get(&cid);
			let state: S = from_cbor(block.data()).expect("state to be dag-cbor");
			state
		},
	};

	// event
	let event_cid = context.event();
	let event_block = context.storage().get(&event_cid);
	let event: ReducerAction<S::Action> = from_cbor(event_block.data()).expect("event to be dag-cbor");

	// reduce
	let next_state = state.reduce(&event, context);

	// store
	let next_data = to_cbor(&next_state).expect("serialize next_state to dag-cbor");
	let next_hash = Code::Blake3_256.digest(&next_data);
	let next_cid = Cid::new_v1(KnownMultiCodec::DagCbor.into(), next_hash);
	let next_block = Block::new_unchecked(next_cid, next_data);
	if cid != Some(next_cid) {
		let store_cid = next_cid;
		context.storage_mut().set(next_block);
		context.store_state(store_cid);
	}
}

pub mod async_reduce {
	use crate::{
		async_api::{Context, Reducer},
		library::wasm_context::WasmContext,
	};
	use cid::Cid;
	use co_primitives::{BlockStorageExt, DiagnosticMessage};
	use futures::{executor::LocalPool, future::LocalBoxFuture, task::LocalSpawnExt, FutureExt};
	use serde::de::DeserializeOwned;
	use std::sync::Arc;

	#[allow(clippy::type_complexity)]
	pub struct ReducerRef<C>(
		Arc<dyn for<'a> Fn(&'a C) -> LocalBoxFuture<'a, Result<Option<Cid>, anyhow::Error>> + Sync + Send + 'static>,
	)
	where
		C: Context + 'static;
	impl<C> ReducerRef<C>
	where
		C: Context + 'static,
	{
		pub fn new<R, A>() -> Self
		where
			R: Reducer<A> + 'static,
			A: Clone + DeserializeOwned + 'static,
		{
			Self(Arc::new(|context| async { reduce_execute_with_context::<R, A, C>(context).await }.boxed_local()))
		}

		pub fn execute(self, context: C) -> C {
			let mut pool = LocalPool::new();
			let handle = pool
				.spawner()
				.spawn_local_with_handle(async move { self.execute_async(context).await })
				.expect("future to execute");
			pool.run_until(handle)
		}

		pub async fn execute_async(&self, mut context: C) -> C {
			match self.reduce(&context).await {
				Ok(next_state) => {
					if let Some(next_state) = next_state {
						context.set_state(next_state);
					}
				},
				Err(err) => {
					let cid = context
						.storage()
						.set_serialized(&DiagnosticMessage::from(err))
						.await
						.expect("DiagnosticMessage to serialize");
					context.write_diagnostic(cid);
				},
			}
			context
		}

		pub async fn reduce(&self, context: &C) -> Result<Option<Cid>, anyhow::Error> {
			(self.0)(context).await
		}
	}
	impl<C> Clone for ReducerRef<C>
	where
		C: Context + 'static,
	{
		fn clone(&self) -> Self {
			Self(self.0.clone())
		}
	}

	pub fn reduce<R, A>()
	where
		R: Reducer<A>,
		A: Clone + DeserializeOwned,
	{
		reduce_with_context::<R, A, _>(WasmContext::new());
	}

	pub fn reduce_with_context<R, A, C>(context: C) -> C
	where
		R: Reducer<A>,
		A: Clone + DeserializeOwned,
		C: Context + 'static,
	{
		let mut pool = LocalPool::new();
		let handle = pool
			.spawner()
			.spawn_local_with_handle(async move { reduce_async_with_context::<R, A, C>(context).await })
			.expect("future to execute");
		pool.run_until(handle)
	}

	pub async fn reduce_async_with_context<R, A, C>(mut context: C) -> C
	where
		R: Reducer<A>,
		A: Clone + DeserializeOwned,
		C: Context + 'static,
	{
		match reduce_execute_with_context::<R, A, C>(&context).await {
			Ok(next_state) => {
				if let Some(next_state) = next_state {
					context.set_state(next_state);
				}
			},
			Err(err) => {
				let cid = context
					.storage()
					.set_serialized(&DiagnosticMessage::from(err))
					.await
					.expect("DiagnosticMessage to serialize");
				context.write_diagnostic(cid);
			},
		}
		context
	}

	pub async fn reduce_execute_with_context<R, A, C>(context: &C) -> Result<Option<Cid>, anyhow::Error>
	where
		R: Reducer<A>,
		A: Clone + DeserializeOwned,
		C: Context + 'static,
	{
		// reduce
		let next_state = R::reduce(context.state().into(), context.event().into(), context.storage()).await?;

		// result
		Ok(next_state.into())
	}
}
