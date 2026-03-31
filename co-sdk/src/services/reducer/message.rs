// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{types::co_reducer_state::CoReducerState, CoStorage};
use co_actor::{Response, ResponseStream};
use co_identity::PrivateIdentityBox;
use co_primitives::{Link, ReducerAction};
use co_storage::OverlayBlockStorage;
use ipld_core::ipld::Ipld;

#[derive(Debug)]
pub enum ReducerMessage {
	/// Get current state.
	State(Response<CoReducerState>),

	/// Subscribe state changes. Upon start the current state is yielded.
	StateStream(ResponseStream<CoReducerState>),

	/// Push action.
	Push(
		Option<OverlayBlockStorage<CoStorage>>,
		CoStorage,
		PrivateIdentityBox,
		Link<ReducerAction<Ipld>>,
		Response<Result<CoReducerState, anyhow::Error>>,
	),

	/// Join state and heads.
	JoinState(
		Option<OverlayBlockStorage<CoStorage>>,
		CoStorage,
		CoReducerState,
		Response<Result<CoReducerState, anyhow::Error>>,
	),

	/// Clear reducer caches.
	Clear(Response<CoReducerState>),
}
