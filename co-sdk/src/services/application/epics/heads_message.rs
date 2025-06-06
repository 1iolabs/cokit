use crate::{
	library::network_identity::network_identity,
	state,
	types::message::heads::{HeadsErrorCode, HeadsMessage},
	Action, CoContext, CoReducer, CoReducerFactory, MappedCoReducerState,
};
use anyhow::anyhow;
use cid::Cid;
use co_actor::Actions;
use co_network::didcomm::EncodedMessage;
use co_primitives::{CoId, Did};
use futures::{future::ready, stream, Stream, StreamExt};
use libp2p::PeerId;
use std::collections::BTreeSet;

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
			if &message.header().message_type == &message_type {
				let heads_message: Option<HeadsMessage> = message.body_deserialize().ok();
				if let Some(heads_message) = heads_message {
					Some((message.sender().cloned(), *peer, message.header().id.clone(), heads_message))
				} else {
					None
				}
			} else {
				None
			}
		},
		_ => None,
	}
	.map(|(from, peer, message_id, message)| Action::HeadsMessageReceived { from, peer, message_id, message })?;
	Some(stream::once(ready(Ok(result))))
}

/// Update CO when receive [`HeadsMessage::Heads`] message.
/// TODO: verify sender/heads?
pub fn heads_message_heads(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::HeadsMessageReceived { from, peer, message_id, message: HeadsMessage::Heads(co, heads) } => Some(
			stream::once(ready((context.clone(), message_id.clone(), from.clone(), *peer, co.clone(), heads.clone())))
				.then(|(context, message_id, from, peer, co, heads)| async move {
					handle_heads(context, message_id, from, peer, co, heads).await
				})
				.flat_map(Action::map_error_stream)
				.map(Ok),
		),
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
		Action::HeadsMessageReceived { from, peer, message_id, message: HeadsMessage::HeadsRequest(co) } => Some(
			stream::once(ready((context.clone(), message_id.clone(), from.clone(), *peer, co.clone())))
				.then(move |(context, message_id, from, peer, co)| async move {
					handle_request_heads(context, message_id, from, peer, co).await
				})
				.map(Action::map_error)
				.map(Ok),
		),
		_ => None,
	}
}

/// See: [`HeadsMessage::Heads`]
#[tracing::instrument(level = tracing::Level::TRACE, skip(context, heads))]
async fn handle_heads(
	context: CoContext,
	message_id: String,
	from: Option<Did>,
	peer: PeerId,
	co: CoId,
	heads: BTreeSet<Cid>,
) -> anyhow::Result<Vec<Action>> {
	let mut actions = Vec::new();
	let co_reducer = context.try_co_reducer(&co).await?;

	// verify
	verify_from_participant(&co_reducer, &from).await?;

	// join
	let previous_state = co_reducer.reducer_state().await;
	let next_state = co_reducer.join(heads.clone()).await?;

	// respond if different
	if previous_state != next_state {
		let body = create_heads_body(&co_reducer).await;
		let message = create_heads_message(&context, &co_reducer, body, Some(message_id), peer).await?;
		actions.push(message);
	}

	// result
	Ok(actions)
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
	let body = match verify_from_participant(&co_reducer, &from).await {
		Ok(_) => create_heads_body(&co_reducer).await,
		Err(err) => {
			tracing::warn!(?err, "co-request-heads-failed");
			HeadsMessage::Error { code: HeadsErrorCode::Forbidden, message: "Forbidden".to_owned() }
		},
	};

	// result
	Ok(create_heads_message(&context, &co_reducer, body, Some(parent_message_id), peer).await?)
}

async fn create_heads_message(
	context: &CoContext,
	co_reducer: &CoReducer,
	body: HeadsMessage,
	parent_message_id: Option<String>,
	to: PeerId,
) -> anyhow::Result<Action> {
	// identity
	let identity = network_identity(&context, &co_reducer, None).await?;

	// message
	let mut header = HeadsMessage::create_header();
	header.thid = parent_message_id;
	let (message_id, message) = EncodedMessage::create_signed_json(&identity, header, &body)?;

	// result
	Ok(Action::DidCommSend { message_id, peer: to, message })
}

async fn create_heads_body(co: &CoReducer) -> HeadsMessage {
	HeadsMessage::Heads(co.id().clone(), MappedCoReducerState::new_co(&co).await.external().heads())
}

async fn verify_from_participant(co_reducer: &CoReducer, from: &Option<Did>) -> anyhow::Result<()> {
	let storage = co_reducer.storage();
	let state = co_reducer.reducer_state().await;

	// verify
	if !state::is_participant(&storage, state.co(), from).await? {
		return Err(anyhow!("Not a participant {:?} of {}", from, co_reducer.id()));
	}

	// result
	Ok(())
}
