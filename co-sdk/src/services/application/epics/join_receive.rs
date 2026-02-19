// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	library::{
		join::{CoJoinPayload, CO_DIDCOMM_JOIN},
		network_identity::network_identity,
	},
	Action, CoContext, CoReducer, CoReducerFactory, CO_CORE_NAME_CO,
};
use anyhow::anyhow;
use co_actor::Actions;
use co_core_co::{CoAction, ParticipantState};
use co_identity::DidCommHeader;
use co_network::PeerId;
use co_primitives::{from_json_string, CloneWithBlockStorageSettings, CoJoin, Did, KnownTag, ReducerAction};
use co_storage::BlockStorageExt;
use futures::{FutureExt, Stream, StreamExt, TryStreamExt};

/// When we receive a join message:
/// - decide is requester is allowed to join
/// - update participant state
///
/// TODO: consensus validation
pub fn join_receive(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidCommReceive { peer, message } => {
			if message.header().message_type == CO_DIDCOMM_JOIN && message.is_validated_sender() {
				let (header, body) = message.clone().into_inner();
				let context = context.clone();
				let peer = *peer;
				Some(
					async move { joined(context, peer, header, body).await }
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

async fn joined(context: CoContext, _peer: PeerId, header: DidCommHeader, body: String) -> anyhow::Result<Vec<Action>> {
	let payload: CoJoinPayload = from_json_string(&body)?;
	let co = context
		.co_reducer(&payload.id)
		.await?
		.ok_or(anyhow!("Unknown CO: {}", payload.id))?;
	let (storage, state) = co.co().await?;
	let join = CoJoin::from_tags(&state.tags).unwrap_or_default();
	let from = header.from.ok_or(anyhow!("invalid header: from"))?.to_string();

	// get identity
	//  find invite identity (if its local)
	let invite_identity_did = find_inviter(&context, &co, &from).await?;
	let identity = network_identity(&context, &co, invite_identity_did.as_ref()).await?;

	// state
	let participant_state = match join {
		CoJoin::Invite => state
			.participants
			.get(&storage, &from)
			.await?
			.filter(|participant| participant.state == ParticipantState::Invite)
			.map(|_| ParticipantState::Active),
		CoJoin::Accept => Some(ParticipantState::Active),
		CoJoin::Did => todo!(),
		CoJoin::Manual => Some(ParticipantState::Pending),
	};

	// apply
	if let Some(participant_state) = &participant_state {
		let action = match participant_state {
			ParticipantState::Active => Some(CoAction::ParticipantJoin { participant: from, tags: Default::default() }),
			ParticipantState::Pending => {
				Some(CoAction::ParticipantPending { participant: from, tags: Default::default() })
			},
			_ => None,
		};
		if let Some(action) = action {
			co.push(&identity, CO_CORE_NAME_CO, &action).await?;
		}
	}

	// // answer with new heads if participant is active
	// let mut result = vec![];
	// if participant_state == Some(ParticipantState::Active) {
	// 	let body = HeadsMessage::Heads(co.id().to_owned(), co.heads().await);
	// 	let mut header = HeadsMessage::create_header();
	// 	header.thid = Some(header.id.clone());
	// 	let (message_id, message) = EncodedMessage::create_signed_json(from, header, body)?;
	// 	result.push(Action::DidCommSend { message_id, peer, message });
	// }
	// Ok(result)

	Ok(vec![])
}

/// Find the inviters identity by walking the log until the first invite action.
async fn find_inviter(context: &CoContext, co: &CoReducer, invited_did: &str) -> Result<Option<String>, anyhow::Error> {
	let storage = co.storage().without_networking();
	let invite_identity_did = context
		.entries_from_heads(co.id(), storage.clone(), co.heads().await)
		.await?
		.try_filter_map(|entry| {
			let storage = storage.clone();
			async move {
				match storage
					.get_deserialized::<ReducerAction<CoAction>>(&entry.entry().payload)
					.await
				{
					Ok(action) if CO_CORE_NAME_CO == action.core => match action.payload {
						CoAction::ParticipantInvite { participant, tags: _ } if participant.as_str() == invited_did => {
							Ok(Some(action.from))
						},
						_ => Ok(None),
					},
					_ => Ok(None),
				}
			}
		})
		.take(1)
		.try_collect::<Vec<Did>>()
		.await
		.ok()
		.and_then(|mut items| items.pop());
	Ok(invite_identity_did)
}
