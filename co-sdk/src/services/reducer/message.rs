use crate::{types::co_reducer_state::CoReducerState, CoStorage};
use cid::Cid;
use co_actor::{Response, ResponseStream};
use co_identity::PrivateIdentityBox;
use co_primitives::{Link, ReducerAction};
use co_storage::OverlayBlockStorage;
use ipld_core::ipld::Ipld;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum ReducerMessage {
	State(Response<CoReducerState>),
	StateStream(ResponseStream<CoReducerState>),

	Push(
		Option<OverlayBlockStorage<CoStorage>>,
		CoStorage,
		PrivateIdentityBox,
		Link<ReducerAction<Ipld>>,
		Response<Result<CoReducerState, anyhow::Error>>,
	),
	JoinHeads(
		Option<OverlayBlockStorage<CoStorage>>,
		CoStorage,
		BTreeSet<Cid>,
		Response<Result<CoReducerState, anyhow::Error>>,
	),
	JoinState(
		Option<OverlayBlockStorage<CoStorage>>,
		CoStorage,
		CoReducerState,
		Response<Result<CoReducerState, anyhow::Error>>,
	),

	/// Clear reducer caches.
	Clear(Response<CoReducerState>),
}
