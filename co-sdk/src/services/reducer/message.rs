// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
