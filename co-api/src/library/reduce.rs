use super::wasm_context::WasmContext;
use crate::{Block, Cid, Context, Reducer, ReducerAction};
use libipld::{
	cbor::DagCborCodec,
	multihash::{Code, MultihashDigest},
};
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
			let state: S = serde_ipld_dagcbor::from_slice(block.data()).expect("state to be dag-cbor");
			state
		},
	};

	// event
	let event_cid = context.event();
	let event_block = context.storage().get(&event_cid);
	let event: ReducerAction<S::Action> =
		serde_ipld_dagcbor::from_slice(event_block.data()).expect("event to be dag-cbor");

	// reduce
	let next_state = state.reduce(&event, context);

	// store
	let next_data = serde_ipld_dagcbor::to_vec(&next_state).unwrap();
	let next_hash = Code::Blake3_256.digest(&next_data);
	let next_cid = Cid::new_v1(DagCborCodec.into(), next_hash);
	let next_block = Block::new_unchecked(next_cid, next_data);
	if cid.is_none() || cid.unwrap() != next_cid {
		let store_cid = next_cid;
		context.storage_mut().set(next_block);
		context.store_state(store_cid);
	}
}
