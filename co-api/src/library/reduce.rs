use super::wasm_context::WasmContext;
use crate::{Block, Cid, Context, Reducer, ReducerAction};
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
	let next_data = to_cbor(&next_state).unwrap();
	let next_hash = Code::Blake3_256.digest(&next_data);
	let next_cid = Cid::new_v1(KnownMultiCodec::DagCbor.into(), next_hash);
	let next_block = Block::new_unchecked(next_cid, next_data);
	if cid.is_none() || cid.unwrap() != next_cid {
		let store_cid = next_cid;
		context.storage_mut().set(next_block);
		context.store_state(store_cid);
	}
}

pub mod async_reduce {
	use crate::{
		async_api::{Context, Reducer},
		library::{wasm_context::WasmContext, wasm_storage::WasmStorage},
	};
	use anyhow::Context as _;
	use cid::Cid;
	use co_primitives::{from_cbor, BlockStorage, ReducerAction};
	use futures::{executor::LocalPool, task::LocalSpawnExt};
	use serde::de::DeserializeOwned;

	pub fn reduce<R, A>()
	where
		R: Reducer<A, WasmStorage>,
		A: Clone + DeserializeOwned,
	{
		let context = WasmContext::new();
		reduce_with_context::<R, A, WasmContext, WasmStorage>(context);
	}

	pub fn reduce_with_context<R, A, C, S>(mut context: C)
	where
		R: Reducer<A, S>,
		A: Clone + DeserializeOwned,
		S: BlockStorage + 'static,
		C: Context<S> + 'static,
	{
		let mut pool = LocalPool::new();
		pool.spawner()
			.spawn_local(async move {
				match reduce_execute_with_context::<R, A, C, S>(&context).await {
					Ok(next_state) => {
						if let Some(next_state) = next_state {
							context.set_state(next_state);
						}
					},
					Err(err) => {
						context.set_error(err);
					},
				}
			})
			.expect("future to execute");
		pool.run();
	}

	pub async fn reduce_execute_with_context<R, A, C, S>(context: &C) -> Result<Option<Cid>, anyhow::Error>
	where
		R: Reducer<A, S>,
		A: Clone + DeserializeOwned,
		S: BlockStorage + 'static,
		C: Context<S> + 'static,
	{
		// event
		let event_cid = context.event();
		let event_block = context.storage().get(&event_cid).await?;
		let event: ReducerAction<A> = from_cbor(event_block.data()).context("deserialize event")?;

		// reduce
		let next_state = R::reduce(context.state().into(), event, context.storage()).await?;

		// result
		Ok(next_state.into())
	}
}
