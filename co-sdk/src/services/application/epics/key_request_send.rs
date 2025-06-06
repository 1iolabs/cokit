use crate::{
	library::{
		key_exchange::{create_key_request_message, KeyRequestPayload, KeyResponsePayload, CO_DIDCOMM_KEY_RESPONSE},
		response_list::ResponseList,
		settings_timeout::settings_timeout,
	},
	services::network::DidCommSendNetworkTask,
	Action, CoContext, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use co_actor::{Actions, Epic};
use co_core_keystore::KeyStoreAction;
use co_core_membership::MembershipsAction;
use co_identity::{DidCommHeader, PrivateIdentityResolver};
use co_primitives::{from_json_string, CoId, Did};
use futures::{future::try_join, stream, Stream, StreamExt};
use libp2p::PeerId;
use std::{
	future::{ready, Future},
	time::Duration,
};

/// When we join an encrypted CO request its key.
/// In: [`Action::JoinSent`]
pub struct KeyRequestSend {
	pending_key_requests: ResponseList<Action, ()>,
}
impl KeyRequestSend {
	pub fn new() -> Self {
		Self { pending_key_requests: Default::default() }
	}
}
impl Epic<Action, (), CoContext> for KeyRequestSend {
	fn epic(
		&mut self,
		_actions: &Actions<Action, (), CoContext>,
		action: &Action,
		state: &(),
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
		// response
		self.pending_key_requests.handle(action, state);

		// handle
		match action {
			Action::JoinSent { co, encrypted, participant, peer } if *encrypted => Some({
				let message_id = DidCommHeader::create_message_id();
				let message_response = self.pending_key_requests.create({
					let message_id = message_id.clone();
					move |action, _| filter_response(&message_id, action)
				});
				stream::once(ready((
					context.clone(),
					message_id,
					message_response,
					co.clone(),
					participant.clone(),
					*peer,
				)))
				.then(move |(context, message_id, message_response, co, participant, peer)| {
					key_request(message_id, message_response, context, peer, co, participant)
				})
				.flat_map(Action::map_error_stream)
				.map(Ok)
			}),
			_ => None,
		}
	}
}

/// Send request and wait for response.
/// TODO: handle error - abort (or set back to invite) the membership?
async fn key_request(
	message_id: String,
	message_response: impl Future<Output = Result<(PeerId, DidCommHeader, String), anyhow::Error>>,
	context: CoContext,
	peer: PeerId,
	co: CoId,
	did: Did,
) -> anyhow::Result<Vec<Action>> {
	if let Some(network) = context.network_tasks().await {
		let timeout = settings_timeout(&context, &CoId::from(CO_ID_LOCAL), Some("key-exchange")).await;

		// from
		let identity = context.private_identity_resolver().await?.resolve_private(&did).await?;

		// message
		let message = create_key_request_message(
			message_id,
			&identity,
			KeyRequestPayload {
				peer: network.local_peer_id(),
				id: co.clone(),
				key: None, // latest
			},
			Duration::from_secs(30 * 60),
		)?;

		// send
		//  note: this expects the connection from join is still opened
		let send = DidCommSendNetworkTask::send(network.clone(), [peer], message, timeout);

		// receive
		let receive = async move { Ok(tokio::time::timeout(timeout, message_response).await??) };

		// execute
		let ((_response_peer, _response_header, body), _) = try_join(receive, send).await?;
		let payload: KeyResponsePayload = from_json_string(&body)?;
		let key = match payload {
			KeyResponsePayload::Ok(key) => key,
			KeyResponsePayload::Failure => Err(anyhow!("Key request failed"))?,
		};
		let key_uri = key.uri.clone();

		// apply
		let local_co = context.local_co_reducer().await?;
		local_co
			.push(&identity, CO_CORE_NAME_KEYSTORE, &KeyStoreAction::Set(key))
			.await?;
		local_co
			.push(
				&identity,
				CO_CORE_NAME_MEMBERSHIP,
				&MembershipsAction::ChangeKey { id: co.clone(), did: did.clone(), key: key_uri },
			)
			.await?;

		// join
		Ok(vec![Action::Joined { co: co.clone(), participant: did, success: true, peer: Some(peer) }])
	} else {
		Ok(Default::default())
	}
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
