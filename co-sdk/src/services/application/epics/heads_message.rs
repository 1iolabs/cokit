use crate::{
	state,
	types::message::heads::{HeadsErrorCode, HeadsMessage},
	Action, CoContext, CoReducerFactory,
};
use anyhow::anyhow;
use co_network::didcomm::EncodedMessage;
use co_primitives::{CoId, Did};
use futures::{future::ready, stream, Stream, StreamExt};
use libipld::Cid;
use libp2p::PeerId;
use std::collections::BTreeSet;

/// Receive [`HeadsMessage`] DIDComm message.
///
/// In: [`Action::DidCommReceive`]
/// Out: [`Action::HeadsMessageReceived`]
pub fn heads_message_receive(
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
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::HeadsMessageReceived { from: _, peer, message_id, message: HeadsMessage::Heads(co, heads) } => Some(
			stream::once(ready((context.clone(), message_id.clone(), *peer, co.clone(), heads.clone())))
				.then(|(context, message_id, peer, co, heads)| async move {
					join_heads(context, message_id, peer, co, heads).await
				})
				.flat_map(Action::map_error_stream)
				.map(Ok),
		),
		_ => None,
	}
}

/// Respond when receive [`HeadsMessage::HeadsRequest`] message.
pub fn heads_message_heads_request(
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::HeadsMessageReceived { from, peer, message_id, message: HeadsMessage::HeadsRequest(co) } => Some(
			stream::once(ready((context.clone(), message_id.clone(), from.clone(), *peer, co.clone())))
				.then(move |(context, message_id, from, peer, co)| async move {
					request_heads(context, message_id, from, peer, co).await
				})
				.map(Action::map_error)
				.map(Ok),
		),
		_ => None,
	}
}

#[tracing::instrument(skip(context, heads))]
async fn join_heads(
	context: CoContext,
	message_id: String,
	peer: PeerId,
	co: CoId,
	heads: BTreeSet<Cid>,
) -> anyhow::Result<Vec<Action>> {
	let mut actions = Vec::new();
	let co_reducer = context.try_co_reducer(&co).await?;
	if co_reducer.join(&heads).await? {
		let next_heads = co_reducer.heads().await;
		if next_heads != heads {
			let mut header = HeadsMessage::create_header();
			header.thid = Some(message_id);
			let body = HeadsMessage::Heads(co, next_heads);
			// TODO: sign?
			let (message_id, message) = EncodedMessage::create_plain_json(header, &body)?;
			actions.push(Action::DidCommSend { message_id, peer, message });
		}
	}
	Ok(actions)
}

async fn request_heads(
	context: CoContext,
	parent_message_id: String,
	from: Option<Did>,
	peer: PeerId,
	co: CoId,
) -> anyhow::Result<Action> {
	let body = match get_heads(&context, &from, &co).await {
		Ok(heads) => HeadsMessage::Heads(co, heads),
		Err(err) => {
			tracing::warn!(?err, "co-request-heads-failed");
			HeadsMessage::Error { code: HeadsErrorCode::Forbidden, message: "Forbidden".to_owned() }
		},
	};
	let mut header = HeadsMessage::create_header();
	header.thid = Some(parent_message_id);
	// TODO: sign?
	let (message_id, message) = EncodedMessage::create_plain_json(header, &body)?;
	Ok(Action::DidCommSend { message_id, peer, message })
}

async fn get_heads(context: &CoContext, from: &Option<Did>, co: &CoId) -> anyhow::Result<BTreeSet<Cid>> {
	let co_reducer = context.try_co_reducer(&co).await?;

	// verify
	if !state::is_participant(&co_reducer.storage(), co_reducer.co_state().await, from).await? {
		return Err(anyhow!("Not a participant {:?} of {}", from, co));
	}

	// result
	Ok(co_reducer.heads().await)
}
