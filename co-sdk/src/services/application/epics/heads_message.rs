// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::{compat::Instant, network_identity::network_identity, shared_membership::shared_membership},
	services::application::{HeadsError, HeadsMessageReceivedAction},
	state, Action, ActionError, CoAccessPolicy, CoContext, CoReducer, CoReducerFactory, MappedCoReducerState,
};
use anyhow::anyhow;
use cid::Cid;
use co_actor::{ActionDispatch, Actions};
use co_core_membership::{Membership, MembershipState};
use co_identity::PeerDidCommHeader;
use co_network::{connections::PeerRelateCoAction, EncodedMessage, HeadsErrorCode, HeadsMessage, PeerId};
use co_primitives::{CoId, Did, WeakCid};
use futures::{future::ready, stream, FutureExt, Stream, StreamExt};
use std::{collections::BTreeSet, str::FromStr};

/// Receive [`HeadsMessage`] DIDComm message.
///
/// In: [`Action::DidCommReceive`]
/// Out: [`Action::HeadsMessageReceived`]
pub fn heads_message_receive(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	_context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	let message_type = HeadsMessage::message_type();
	let result = match action {
		Action::DidCommReceive { peer, message } => {
			if message.header().message_type == message_type {
				let heads_message: Option<HeadsMessage> = message.body_deserialize().ok();
				if let Some(heads_message) = heads_message {
					let header = PeerDidCommHeader::from(message.header().clone());
					let from_peer = header.from_peer_id.and_then(|s| PeerId::from_str(&s).ok());
					Some((message.sender().cloned(), *peer, from_peer, message.header().id.clone(), heads_message))
				} else {
					None
				}
			} else {
				None
			}
		},
		_ => None,
	}
	.map(|(from, peer, from_peer, message_id, message)| {
		Action::HeadsMessageReceived(HeadsMessageReceivedAction {
			co: message.co().clone(),
			from,
			from_peer,
			peer,
			message_id,
			message,
			tags: Default::default(),
		})
	})?;
	Some(stream::once(ready(Ok(result))))
}

/// Update CO when receive [`HeadsMessage::Heads`] message.
/// TODO: verify sender/heads?
pub fn heads_message_heads(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::HeadsMessageReceived(
			message @ HeadsMessageReceivedAction { message: HeadsMessage::Heads(co, heads), .. },
		) => Some(handle_heads(actions.clone(), context.clone(), message.clone(), co.clone(), heads.clone())),
		_ => None,
	}
}

/// Respond when receive [`HeadsMessage::HeadsRequest`] message.
pub fn heads_message_heads_request(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::HeadsMessageReceived(HeadsMessageReceivedAction {
			from,
			peer,
			message_id,
			message: HeadsMessage::HeadsRequest(co),
			..
		}) => Some({
			let context = context.clone();
			let message_id = message_id.clone();
			let from = from.clone();
			let peer = *peer;
			let co = co.clone();
			async move { handle_request_heads(context, message_id, from, peer, co).await }
				.into_stream()
				.map(Action::map_error)
				.map(Ok)
		}),
		_ => None,
	}
}

/// See: [`HeadsMessage::Heads`]
fn handle_heads(
	actions: Actions<Action, (), CoContext>,
	context: CoContext,
	action: HeadsMessageReceivedAction,
	co: CoId,
	heads: BTreeSet<WeakCid>,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	ActionDispatch::execute_with_response(
		actions,
		context.tasks(),
		{
			let action = action.clone();
			move |dispatch| async move {
				// verify we got a active membership for this co
				let local_co = context.local_co_reducer().await?;
				match shared_membership(&local_co, &co, None).await? {
					Some(Membership { membership_state: MembershipState::Active, .. }) => {
						// active -> ok
					},
					Some(Membership { membership_state: MembershipState::Invite | MembershipState::Join, .. }) => {
						// pending -> queue
						return Err(HeadsError::Transient(ActionError::from(anyhow!("Pending membership"))));
					},
					_ => {
						// no membership -> ignore
						return Ok(());
					},
				}

				// reducer
				let co_reducer = context.try_co_reducer(&co).await.map_err(anyhow::Error::from)?;

				// verify
				verify_from_participant(&context, &co_reducer, &action.from)
					.await
					.map_err(|err| HeadsError::Permanent(err.into()))?;

				// network: let others know that the Co is connected and allow to use the implicit direct peer
				// connection
				if let Some(from_peer_id) = action.from_peer {
					if let Some(connections) = context.network_connections().await {
						connections
							.dispatch(PeerRelateCoAction {
								co: co.clone(),
								peer_id: from_peer_id,
								did: action.from.clone(),
								time: Instant::now(),
							})
							.ok();
					}
				}

				// join
				let previous_state = co_reducer.reducer_state().await;
				let next_state = co_reducer.join(heads.into_iter().map(Cid::from).collect()).await?;

				// respond if different
				if previous_state != next_state {
					let body = create_heads_body(&co_reducer).await;
					let message =
						create_heads_message(&context, &co_reducer, body, Some(action.message_id), action.peer).await?;
					dispatch.dispatch(message);
				}

				// result
				Ok(())
			}
		},
		move |result: Result<(), HeadsError>| Action::HeadsMessageComplete(action, result),
	)
}

/// See: [`HeadsMessage::HeadsRequest`]
async fn handle_request_heads(
	context: CoContext,
	parent_message_id: String,
	from: Option<Did>,
	peer: PeerId,
	co: CoId,
) -> anyhow::Result<Action> {
	// identity
	let co_reducer = context.try_co_reducer(&co).await?;

	// body
	let body = match verify_from_participant(&context, &co_reducer, &from).await {
		Ok(_) => create_heads_body(&co_reducer).await,
		Err(err) => {
			tracing::warn!(?co, ?peer, ?from, ?err, "co-request-heads-failed");
			HeadsMessage::Error { co, code: HeadsErrorCode::Forbidden, message: "Forbidden".to_owned() }
		},
	};

	// result
	create_heads_message(&context, &co_reducer, body, Some(parent_message_id), peer).await
}

async fn create_heads_message(
	context: &CoContext,
	co_reducer: &CoReducer,
	body: HeadsMessage,
	parent_message_id: Option<String>,
	to: PeerId,
) -> anyhow::Result<Action> {
	// identity
	let identity = network_identity(context, co_reducer, None).await?;

	// message
	let mut header = HeadsMessage::create_header(context.date());
	header.thid = parent_message_id;
	let (message_header, message) = EncodedMessage::create_signed_json(&identity, header, &body)?;

	// result
	Ok(Action::DidCommSend { message_header, peer: to, message })
}

async fn create_heads_body(co: &CoReducer) -> HeadsMessage {
	HeadsMessage::Heads(co.id().clone(), MappedCoReducerState::new_co(co).await.external().weak_heads())
}

/// Respond when receive [`HeadsMessage::StateRequest`] message.
pub fn heads_message_state_request(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::HeadsMessageReceived(HeadsMessageReceivedAction {
			from,
			peer,
			message_id,
			message: HeadsMessage::StateRequest(co),
			..
		}) => Some({
			let context = context.clone();
			let message_id = message_id.clone();
			let from = from.clone();
			let peer = *peer;
			let co = co.clone();
			async move { handle_request_state(context, message_id, from, peer, co).await }
				.into_stream()
				.map(Action::map_error)
				.map(Ok)
		}),
		_ => None,
	}
}

/// See: [`HeadsMessage::StateRequest`]
async fn handle_request_state(
	context: CoContext,
	parent_message_id: String,
	from: Option<Did>,
	peer: PeerId,
	co: CoId,
) -> anyhow::Result<Action> {
	// identity
	let co_reducer = context.try_co_reducer(&co).await?;

	// body
	let body = match verify_from_participant(&context, &co_reducer, &from).await {
		Ok(_) => create_state_body(&co_reducer).await?,
		Err(err) => {
			tracing::warn!(?co, ?peer, ?from, ?err, "co-request-state-failed");
			HeadsMessage::Error { co, code: HeadsErrorCode::Forbidden, message: "Forbidden".to_owned() }
		},
	};

	// result
	create_heads_message(&context, &co_reducer, body, Some(parent_message_id), peer).await
}

async fn create_state_body(co: &CoReducer) -> anyhow::Result<HeadsMessage> {
	let (state, heads) = MappedCoReducerState::new_co(co).await.external().weak();
	Ok(HeadsMessage::State(
		co.id().clone(),
		state.ok_or_else(|| anyhow!("no state"))?,
		heads,
	))
}

async fn verify_from_participant(
	context: &CoContext,
	co_reducer: &CoReducer,
	from: &Option<Did>,
) -> anyhow::Result<()> {
	let storage = co_reducer.storage();
	let state = co_reducer.reducer_state().await;

	// verify
	if !state::is_participant(&storage, state.co(), from).await? {
		if let Some(did) = from {
			match context.access_policy() {
				Some(policy) if policy.check_access(co_reducer.id(), did).await? => return Ok(()),
				_ => {},
			}
		}
		return Err(anyhow!("Not a participant {:?} of {}", from, co_reducer.id()));
	}

	// result
	Ok(())
}
