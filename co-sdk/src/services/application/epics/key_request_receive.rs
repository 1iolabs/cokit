use crate::{
	library::{
		find_co_secret::find_co_key,
		key_exchange::{create_key_response_message, KeyRequestPayload, KeyResponsePayload, CO_DIDCOMM_KEY_REQUEST},
		network_identity::network_identity,
	},
	Action, CoContext, CoReducerFactory,
};
use anyhow::anyhow;
use co_actor::Actions;
use co_core_keystore::Key;
use co_identity::{DidCommHeader, Identity, IdentityResolver};
use co_primitives::from_json_string;
use futures::{FutureExt, Stream, StreamExt};
use libp2p::PeerId;

/// When we receive an key request send an response.
pub fn key_request_receive(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidCommReceive { peer, message } => {
			if &message.header().message_type == CO_DIDCOMM_KEY_REQUEST && message.is_validated_sender() {
				let (header, body) = message.clone().into_inner();
				let context = context.clone();
				let peer = *peer;
				Some(
					async move { key_request(context, peer, header, body).await }
						.into_stream()
						.flat_map(Action::map_error_stream)
						.map(Ok),
				)
			} else {
				None
			}
		},
		_ => None,
	}
}

async fn key_request(
	context: CoContext,
	peer: PeerId,
	header: DidCommHeader,
	body: String,
) -> anyhow::Result<Vec<Action>> {
	// payload
	let payload: KeyRequestPayload = from_json_string(&body)?;
	if payload.peer != peer {
		// SECURITY:
		//  to mitigate MITM we only accept this request if its from the peer that is has been signed for
		return Err(anyhow!("invalid payload"));
	}
	let from = header.from.ok_or(anyhow!("invalid header: from"))?.to_string();

	// requester
	let identity_resolver = context.identity_resolver().await?;
	let requester_identity = identity_resolver.resolve(&from).await?;

	// get participant state
	//  we only allow active participants to request keys
	let co = context.try_co_reducer(&payload.id).await?;
	let (_storage, co_state) = co.co().await?;
	let participant_state = co_state
		.participants
		.get(requester_identity.identity())
		.map(|participant| participant.state)
		.unwrap_or(co_core_co::ParticipantState::Inactive);

	// validate access
	if !participant_state.has_access() {
		return Err(anyhow!("Invalid participant state: {:?}", participant_state));
	}

	// membership
	//  we use any network identity for the co as we don't know easliy who is the inviter.
	let identity = network_identity(&context, &co, None).await?;

	// key
	let local_co = context.local_co_reducer().await?;
	let key: Key = find_co_key(&local_co, &co).await?.ok_or(anyhow!("No key found"))?;

	// message
	let (message_id, message) =
		create_key_response_message(&identity, &requester_identity, header.id.clone(), KeyResponsePayload::Ok(key))?;

	// result
	Ok(vec![Action::DidCommSend { message_id, peer, message }])
}
