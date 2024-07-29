use crate::{
	drivers::network::tasks::didcomm_send::DidCommSendNetworkTask,
	library::{
		find_co_identities::find_co_private_identity,
		find_co_secret::find_co_key,
		key_exchange::{create_key_response_message, KeyRequestPayload, KeyResponsePayload, CO_DIDCOMM_KEY_REQUEST},
		settings_timeout::settings_timeout,
	},
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CoReducerFactory,
};
use anyhow::anyhow;
use co_core_co::ParticipantState;
use co_core_keystore::Key;
use co_identity::{DidCommHeader, Identity, IdentityResolver};
use futures::{Stream, StreamExt};
use libp2p::PeerId;
use std::{future::ready, time::Duration};

/// When we receive an key request send an response.
pub fn key_request_receive(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| {
			ready(match action {
				Action::DidCommReceive { peer, message } => {
					if &message.header().message_type == CO_DIDCOMM_KEY_REQUEST
						&& message.header().to.len() == 1
						&& message.is_validated_sender()
					{
						let (header, body) = message.into_inner();
						Some((peer, header, body))
					} else {
						None
					}
				},
				_ => None,
			})
		})
		.then(move |(peer, header, body)| {
			let context = context.clone();
			async move { key_request(context, peer, header, body).await }
		})
		.flat_map(Action::map_error_stream)
}

async fn key_request(
	context: CoContext,
	peer: PeerId,
	header: DidCommHeader,
	body: String,
) -> anyhow::Result<Vec<Action>> {
	let network = context.network().await.ok_or(anyhow!("Expected network"))?;

	// payload
	let payload: KeyRequestPayload = serde_json::from_str(&body)?;
	if payload.peer != peer {
		return Err(anyhow!("invalid payload"));
	}
	let from = header.from.ok_or(anyhow!("invalid header: from"))?.to_string();

	// requester
	let identity_resolver = context.identity_resolver().await?;
	let requester_identity = identity_resolver.resolve(&from).await?;

	// get participant state
	//  we only allow active participants to request keys
	let co = context
		.co_reducer(&payload.id)
		.await?
		.ok_or(anyhow!("Co not found: {}", payload.id))?;
	let co_state = co.co().await?;
	let participant_state = co_state
		.participants
		.get(requester_identity.identity())
		.map(|participant| participant.state)
		.unwrap_or(co_core_co::ParticipantState::Inactive);

	// send response
	if participant_state != ParticipantState::Active {
		return Err(anyhow!("Invalid participant state: {:?}", participant_state));
	}

	// membership
	//  we currently use the identity of the first membership we found for an co to send the response
	//  it should not metter which identity we use as the receive then knows about all the participants
	let identity = find_co_private_identity(&context, &payload.id).await?;

	// key
	let local_co = context.local_co_reducer().await?;
	let key: Key = find_co_key(&local_co, &co).await?.ok_or(anyhow!("No key found"))?;

	// message
	let message =
		create_key_response_message(&identity, &requester_identity, header.id.clone(), KeyResponsePayload::Ok(key))?;

	// timeout
	let timeout: Duration = settings_timeout(&context, &payload.id, Some("key-exchange")).await;

	// send
	DidCommSendNetworkTask::send(network.clone(), [peer], message, timeout).await?;

	// result
	Ok(vec![])
}
