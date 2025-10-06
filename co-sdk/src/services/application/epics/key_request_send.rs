use crate::{
	library::{
		key_exchange::{create_key_request_message, KeyRequestPayload, KeyResponsePayload, CO_DIDCOMM_KEY_RESPONSE},
		network_identity::network_identity_by_id,
		network_queue::TaskState,
	},
	services::application::{action::KeyRequestAction, CoDidCommSendAction},
	Action, ActionError, CoContext, CoNetworkTaskSpawner, CoReducerFactory, CO_CORE_NAME_KEYSTORE,
	CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use co_actor::{ActionDispatch, Actions};
use co_core_keystore::KeyStoreAction;
use co_core_membership::MembershipsAction;
use co_identity::{DidCommHeader, Identity, PrivateIdentityResolver};
use co_primitives::{from_json_string, BlockSerializer, CoId};
use futures::{future::Either, stream, FutureExt, Stream, StreamExt};
use libp2p::PeerId;
use std::time::Duration;

const NETWORK_QUEUE_TYPE: &str = "network-key";

/// Key request.
///
/// In: [`Action::KeyRequest`]
/// Out: [`Action::CoDidCommSend`], [`Action::KeyRequestCompleted`]
pub fn key_request_send(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::KeyRequest(action) => {
			let action = action.clone();
			let actions = actions.clone();
			let context = context.clone();
			Some(
				async move {
					// network
					let network = context.network().await;
					let Some((network, _connections)) = network else {
						return Either::Left(stream::iter([Action::network_task_queue(
							action.co.clone(),
							format!("urn:key:{}:{}", action.co, action.key.as_deref().unwrap_or("")),
							NETWORK_QUEUE_TYPE,
							format!("Key {} for co:{}", action.key.as_deref().unwrap_or("[latest]"), action.co),
							&action,
						)]));
					};

					// send
					Either::Right(handle_key_request(context.clone(), network, actions, action.clone()))
				}
				.into_stream()
				.flatten(),
			)
		},
		_ => None,
	}
}

fn handle_key_request(
	context: CoContext,
	network: CoNetworkTaskSpawner,
	actions: Actions<Action, (), CoContext>,
	action: KeyRequestAction,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	ActionDispatch::execute_with_response(
		actions,
		context.tasks(),
		{
			let action = action.clone();
			move |dispatch| async move {
				let from =
					network_identity_by_id(&context, &action.parent_co, &action.co, action.from.as_ref()).await?;

				// message
				let (message_id, message) = create_key_request_message(
					&from,
					KeyRequestPayload { peer: network.local_peer_id(), id: action.co.clone(), key: action.key },
					Duration::from_secs(30 * 60),
				)?;

				// response
				let (_response_peer, _response_header, response_body) = dispatch
					.request(
						Action::CoDidCommSend(CoDidCommSendAction {
							co: action.co.clone(),
							networks: action.network.unwrap_or_default(),
							notification: None,
							tags: Default::default(),
							message_from: from.identity().to_string(),
							message_id: message_id.clone(),
							message,
						}),
						move |action| filter_response(&message_id, action),
					)
					.await?;

				// load
				let payload: KeyResponsePayload = from_json_string(&response_body)?;
				let key = match payload {
					KeyResponsePayload::Ok(key) => key,
					KeyResponsePayload::Failure => Err(anyhow!("Key request failed"))?,
				};
				let key_uri = key.uri.clone();

				// apply
				let parent_co = context.try_co_reducer(&action.parent_co).await?;
				parent_co.push(&from, CO_CORE_NAME_KEYSTORE, &KeyStoreAction::Set(key)).await?;
				parent_co
					.push(
						&from,
						CO_CORE_NAME_MEMBERSHIP,
						&MembershipsAction::ChangeKey {
							id: action.co.clone(),
							did: from.identity().to_owned(),
							key: key_uri.clone(),
						},
					)
					.await?;

				Ok(key_uri)
			}
		},
		move |result: Result<String, anyhow::Error>| {
			Action::KeyRequestComplete(action, result.map_err(ActionError::from))
		},
	)
}

/// Execute queued [`NetworkBlockGetAction`].
///
/// In: [`Action::NetworkTaskExecute`]
/// Out: [`Action::NetworkBlockGetComplete`], [`Action::NetworkTaskExecuteComplete`]
pub fn network_task_execute(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkTaskExecute { co, task_id, task_type, task } if task_type == NETWORK_QUEUE_TYPE => {
			let action = BlockSerializer::default().deserialize::<KeyRequestAction>(task);
			let context = context.clone();
			let task_id = task_id.clone();
			let co = co.clone();
			let actions = actions.clone();
			Some(
				async move {
					// action
					let Ok(action) = action else {
						return Either::Left(stream::iter([Ok(Action::NetworkTaskExecuteComplete {
							co,
							task_id,
							task_state: TaskState::Failed,
						})]));
					};

					// network
					let network = context.network().await;
					let Some((network, _connections)) = network else {
						return Either::Left(stream::iter([Ok(Action::NetworkTaskExecuteComplete {
							co,
							task_id,
							task_state: TaskState::Failed,
						})]));
					};

					// send
					Either::Right(handle_key_request(context.clone(), network, actions, action.clone()).flat_map(
						move |item| match item {
							Ok(Action::KeyRequestComplete(request, result)) => stream::iter(vec![
								Ok(Action::NetworkTaskExecuteComplete {
									co: action.co.clone(),
									task_id: task_id.clone(),
									task_state: match result {
										Ok(_) => TaskState::Done,
										Err(_) => TaskState::Failed,
									},
								}),
								Ok(Action::KeyRequestComplete(request, result)),
							]),
							item => stream::iter(vec![item]),
						},
					))
				}
				.into_stream()
				.flatten(),
			)
		},
		_ => None,
	}
}

/// When we join an encrypted CO request its key.
///
/// In: [`Action::JoinKeyRequest`]
/// Out: [`Action::CoDidCommSend`], [`Action::Joined`]
///
/// TODO: Handle DidCommSent without and peers.
/// TODO: Add timeout?
/// TODO: When to retry?
/// TODO: handle error - abort (or set back to invite) the membership?
pub fn join_key_request_send(
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
