use crate::{
	library::join::{CoJoinPayload, CO_DIDCOMM_JOIN},
	Action, CoContext, CoReducerFactory, KnownTag, CO_CORE_NAME_CO,
};
use anyhow::anyhow;
use co_core_co::{CoAction, ParticipantState};
use co_identity::DidCommHeader;
use co_primitives::{from_json_string, CoJoin};
use futures::{future::ready, stream, Stream, StreamExt};
use libp2p::PeerId;

/// When we receive a join message:
/// - decide is requester is allowed to join
/// - update participant state
///
/// TODO: consensus validation
pub fn join_receive(
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidCommReceive { peer, message } => {
			if &message.header().message_type == CO_DIDCOMM_JOIN && message.is_validated_sender() {
				let (header, body) = message.clone().into_inner();
				Some(
					stream::once(ready((context.clone(), *peer, header, body)))
						.then(
							move |(context, peer, header, body)| async move { joined(context, peer, header, body).await },
						)
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
	let (_storage, state) = co.co().await?;
	let join = CoJoin::from_tags(&state.tags).unwrap_or_default();
	let from = header.from.ok_or(anyhow!("invalid header: from"))?.to_string();

	// state
	let participant_state = match join {
		CoJoin::Invite => state
			.participants
			.get(&from)
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
			co.push(&context.local_identity(), CO_CORE_NAME_CO, &action).await?;
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
