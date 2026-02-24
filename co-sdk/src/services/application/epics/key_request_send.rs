// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::{
		key_exchange::{create_key_request_message, KeyRequestPayload, KeyResponsePayload, CO_DIDCOMM_KEY_RESPONSE},
		network_identity::network_identity_by_id,
		network_queue::TaskState,
	},
	services::application::{action::KeyRequestAction, CoDidCommSendAction},
	Action, ActionError, CoContext, CoReducerFactory, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use co_actor::{ActionDispatch, Actions};
use co_core_keystore::KeyStoreAction;
use co_core_membership::MembershipsAction;
use co_identity::{DidCommHeader, Identity};
use co_network::{NetworkApi, PeerId};
use co_primitives::{from_json_string, BlockSerializer};
use futures::{future::Either, stream, FutureExt, Stream, StreamExt};
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
					let Some(network) = context.network().await else {
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
	network: NetworkApi,
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
				let (message_header, message) = create_key_request_message(
					context.date(),
					&from,
					KeyRequestPayload { peer: network.local_peer_id(), id: action.co.clone(), key: action.key },
					Duration::from_secs(30 * 60),
				)?;
				let message_id = message_header.id.clone();

				// response
				let (_response_peer, _response_header, response_body) = dispatch
					.request(
						Action::CoDidCommSend(CoDidCommSendAction {
							co: action.co.clone(),
							networks: action.network.unwrap_or_default(),
							notification: None,
							tags: Default::default(),
							message_from: from.identity().to_string(),
							message_header,
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
					let Some(network) = context.network().await else {
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

fn filter_response(message_id: &str, action: &Action) -> Option<(PeerId, DidCommHeader, String)> {
	match action {
		Action::DidCommReceive { peer, message } => {
			if message.header().message_type == CO_DIDCOMM_KEY_RESPONSE
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
