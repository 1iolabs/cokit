use super::FlushInfo;
use crate::{types::co_reducer_state::CoReducerState, CoStorage};
use cid::Cid;
use co_actor::{Response, ResponseStream};
use co_identity::PrivateIdentityBox;
use co_primitives::{Link, ReducerAction};
use ipld_core::ipld::Ipld;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum ReducerMessage {
	State(Response<CoReducerState>),
	StateStream(ResponseStream<CoReducerState>),

	Push(PrivateIdentityBox, CoStorage, Link<ReducerAction<Ipld>>, Response<Result<CoReducerState, anyhow::Error>>),
	JoinHeads(CoStorage, BTreeSet<Cid>, Response<Result<CoReducerState, anyhow::Error>>),
	JoinState(CoStorage, CoReducerState, Response<Result<CoReducerState, anyhow::Error>>),

	/// Flush staged changes to disk.
	Flush(CoStorage, Response<Result<Option<FlushInfo>, anyhow::Error>>),

	/// Clear reducer caches.
	Clear(Response<CoReducerState>),
}
