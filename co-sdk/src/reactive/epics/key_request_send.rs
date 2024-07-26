use crate::{
	drivers::network::tasks::didcomm_send::DidCommSendNetworkTask,
	library::{
		create_reducer_action::create_reducer_action,
		key_exchange::{create_key_request_message, KeyRequestPayload, KeyResponsePayload, CO_DIDCOMM_KEY_RESPONSE},
		settings_timeout::settings_timeout,
	},
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CO_CORE_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use co_core_keystore::KeyStoreAction;
use co_core_membership::MembershipsAction;
use co_identity::{DidCommHeader, PrivateIdentityResolver};
use co_primitives::{CoId, Did, MultiCodec};
use futures::{future::try_join, pin_mut, Stream, StreamExt};
use libipld::Cid;
use libp2p::PeerId;
use std::{future::ready, time::Duration};

/// When we join an encrypted CO request its key.
pub fn key_request_send(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.clone()
		.filter_map(|action| {
			ready(match action {
				Action::JoinSent { co, heads, participant, peer } if is_encrypted(&heads) => {
					Some((co, participant, peer))
				},
				_ => None,
			})
		})
		.then(move |(co, participant, peer)| key_request(actions.clone(), context.clone(), peer, co, participant))
		.flat_map(Action::map_error_stream)
}

fn is_encrypted<'a>(cids: impl IntoIterator<Item = &'a Cid>) -> bool {
	for cid in cids {
		match MultiCodec::from(cid) {
			MultiCodec::CoEncryptedBlock => {
				return true;
			},
			_ => {},
		}
	}
	return false;
}

/// Send request and wait for response.
/// TODO: handle error - abort (or set back to invite) the membership?
async fn key_request(
	actions: ActionObservable,
	context: CoContext,
	peer: PeerId,
	co: CoId,
	did: Did,
) -> anyhow::Result<Vec<Action>> {
	if let Some(network) = context.network().await {
		let timeout = settings_timeout(&context, &co, Some("key-exchange")).await;

		// from
		let identity = context.private_identity_resolver().await?.resolve_private(&did).await?;

		// message
		let (message_id, message) = create_key_request_message(
			&identity,
			KeyRequestPayload {
				peer: network.local_peer_id(),
				id: co.clone(),
				key: None, // latest
			},
		)?;

		// send
		//  note: this expects the connection from join is still opened
		let send = DidCommSendNetworkTask::send(network.clone(), [peer], message.clone(), timeout);

		// receive
		let receive = wait_response(actions, message_id, timeout);

		// execute
		let ((_response_peer, _response_header, body), _) = try_join(receive, send).await?;
		let payload: KeyResponsePayload = serde_json::from_str(&body)?;
		let key = match payload {
			KeyResponsePayload::Ok(key) => key,
			KeyResponsePayload::Failure => Err(anyhow!("Key request failed"))?,
		};

		// process
		let change = create_reducer_action(
			&did,
			CO_CORE_NAME_MEMBERSHIP,
			MembershipsAction::ChangeKey { id: co.clone(), did: did.clone(), key: key.uri.clone() },
		)?;
		let set = create_reducer_action(&did, CO_CORE_KEYSTORE, KeyStoreAction::Set(key))?;
		Ok(vec![
			Action::CoreActionPush { co: CO_ID_LOCAL.into(), action: set },
			Action::CoreActionPush { co: CO_ID_LOCAL.into(), action: change },
			Action::Joined { co: co.clone(), participant: did, success: true },
		])
	} else {
		Ok(Default::default())
	}
}

async fn wait_response(
	actions: ActionObservable,
	message_id: String,
	timeout: Duration,
) -> anyhow::Result<(PeerId, DidCommHeader, String)> {
	let stream = actions.filter_map(|action| {
		ready(match action {
			Action::DidCommReceive { peer, message } => {
				if &message.header().message_type == CO_DIDCOMM_KEY_RESPONSE
					&& message.header().to.len() == 1
					&& message.is_validated_sender()
					&& message.header().thid.as_ref() == Some(&message_id)
				{
					let (header, body) = message.into_inner();
					Some((peer, header, body))
				} else {
					None
				}
			},
			_ => None,
		})
	});
	let stream = tokio_stream::StreamExt::timeout(stream, timeout);
	pin_mut!(stream);
	Ok(stream.next().await.ok_or(anyhow!("No response"))??)
}
