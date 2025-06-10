use crate::{
	library::key_exchange::{
		create_key_request_message, KeyRequestPayload, KeyResponsePayload, CO_DIDCOMM_KEY_RESPONSE,
	},
	Action, CoContext, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use co_actor::Actions;
use co_core_keystore::KeyStoreAction;
use co_core_membership::MembershipsAction;
use co_identity::{DidCommHeader, PrivateIdentityResolver};
use co_primitives::{from_json_string, CoId};
use futures::Stream;
use libp2p::PeerId;
use std::time::Duration;

/// When we join an encrypted CO request its key.
/// In: [`Action::JoinKeyRequest`]
/// Out: [`Action::CoDidCommSend`]
pub fn key_request_send(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::JoinKeyRequest { co, participant, peer } => Some({
			let actions = actions.clone();
			let context = context.clone();
			let co = co.clone();
			let participant = participant.clone();
			let peer = *peer;
			async_stream::try_stream! {
				if let Some((message_id, action)) = create_message(&context, peer, &co, &participant).await? {
					// register response
					let response = actions.once_map(move |action| filter_response(&message_id, action));

					// send
					yield action;

					// response
					if let (response_peer, response_header, response_body) = response.await? {
						for action in key_request_response(&context, &co, &participant, response_peer, response_header, response_body).await? {
							yield action;
						}
					}
				}
			}
		}),
		_ => None,
	}
}

/// TODO: handle error - abort (or set back to invite) the membership?
async fn key_request_response(
	context: &CoContext,
	co: &CoId,
	from: &String,
	response_peer: PeerId,
	_response_header: DidCommHeader,
	response_body: String,
) -> anyhow::Result<Vec<Action>> {
	let payload: KeyResponsePayload = from_json_string(&response_body)?;
	let key = match payload {
		KeyResponsePayload::Ok(key) => key,
		KeyResponsePayload::Failure => Err(anyhow!("Key request failed"))?,
	};
	let key_uri = key.uri.clone();

	// apply
	let local_co = context.local_co_reducer().await?;
	let local_identity = context.local_identity();
	local_co
		.push(&local_identity, CO_CORE_NAME_KEYSTORE, &KeyStoreAction::Set(key))
		.await?;
	local_co
		.push(
			&local_identity,
			CO_CORE_NAME_MEMBERSHIP,
			&MembershipsAction::ChangeKey { id: co.clone(), did: from.clone(), key: key_uri },
		)
		.await?;

	// join
	Ok(vec![Action::Joined { co: co.clone(), participant: from.clone(), success: true, peer: Some(response_peer) }])
}

async fn create_message(
	context: &CoContext,
	join_to: PeerId,
	co: &CoId,
	from: &String,
) -> Result<Option<(String, Action)>, anyhow::Error> {
	// get local peer from network
	//  note: because no join could be sent if we have no network we should never see a [`Action::JoinKeyRequest`] in
	//   first place.
	let local_peer = match context.network_tasks().await {
		Some(network) => network.local_peer_id(),
		None => return Ok(None),
	};
	let from_identity = context.private_identity_resolver().await?.resolve_private(from).await?;
	let (message_id, message) = create_key_request_message(
		&from_identity,
		KeyRequestPayload {
			peer: local_peer,
			id: co.clone(),
			key: None, // latest
		},
		Duration::from_secs(30 * 60),
	)?;
	Ok(Some((message_id.clone(), Action::DidCommSend { message_id, peer: join_to, message })))
}

fn filter_response(message_id: &str, action: &Action) -> Option<(PeerId, DidCommHeader, String)> {
	match action {
		Action::DidCommReceive { peer, message } => {
			if &message.header().message_type == CO_DIDCOMM_KEY_RESPONSE
				&& message.header().to.len() == 1
				&& message.is_validated_sender()
				&& message.header().thid.as_deref() == Some(message_id)
			{
				let (header, body) = message.clone().into_inner();
				Some((*peer, header, body))
			} else {
				None
			}
		},
		_ => None,
	}
}
