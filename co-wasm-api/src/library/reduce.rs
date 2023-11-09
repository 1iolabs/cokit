use crate::{Block, Cid, Context, Reducer, ReducerAction};
use libipld::{
	cbor::DagCborCodec,
	multihash::{Code, MultihashDigest},
};
use serde::{de::DeserializeOwned, Serialize};

pub fn reduce<S>()
where
	S: Reducer + Serialize + DeserializeOwned,
	S::Action: DeserializeOwned,
{
	let mut context = Context::new();

	// state
	let cid = context.state();
	let block = context.storage().get(&cid);
	let state: S = serde_ipld_dagcbor::from_slice(block.data()).expect("state to be cbor");

	// event
	let event_cid = context.event();
	let event_block = context.storage().get(&event_cid);
	let event: ReducerAction<S::Action> = serde_ipld_dagcbor::from_slice(event_block.data()).expect("event to be cbor");

	// reduce
	let next_state = state.reduce(&event, &context);

	// store
	let next_data = serde_ipld_dagcbor::to_vec(&next_state).unwrap();
	let next_hash = Code::Blake3_256.digest(&next_data);
	let next_cid = Cid::new_v1(DagCborCodec.into(), next_hash);
	let next_block = Block::new_unchecked(next_cid, next_data);
	if cid != next_cid {
		let store_cid = next_cid.clone();
		context.storage_mut().set(next_block);
		context.store_state(&store_cid);
	}
}
