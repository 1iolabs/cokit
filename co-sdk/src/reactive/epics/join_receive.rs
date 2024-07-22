use crate::{
	library::join::{CoJoinPayload, CO_DIDCOMM_JOIN},
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CoReducerFactory, KnownTag, CO_CORE_CO,
};
use anyhow::anyhow;
use co_core_co::{CoAction, ParticipantState};
use co_identity::DidCommHeader;
use co_primitives::CoJoin;
use futures::{future::ready, Stream, StreamExt};
use libp2p::PeerId;

/// When we receive a join message:
/// - decide is requester is allowed to join
/// - update participant state
///
/// TODO: consensus validation
pub fn join_receive(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| {
			ready(match action {
				Action::DidCommReceive { peer, message } => {
					if &message.header().message_type == CO_DIDCOMM_JOIN
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
			async move { joined(context, peer, header, body).await }
		})
		.flat_map(Action::map_error_stream)
}

async fn joined(context: CoContext, _peer: PeerId, header: DidCommHeader, body: String) -> anyhow::Result<Vec<Action>> {
	let payload: CoJoinPayload = serde_json::from_str(&body)?;
	let co = context
		.co_reducer(&payload.id)
		.await?
		.ok_or(anyhow!("Unknown CO: {}", payload.id))?;
	let state = co.co().await?;
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
	if let Some(participant_state) = participant_state {
		let action = match participant_state {
			ParticipantState::Active => Some(CoAction::ParticipantJoin { participant: from, tags: Default::default() }),
			ParticipantState::Pending => {
				Some(CoAction::ParticipantPending { participant: from, tags: Default::default() })
			},
			_ => None,
		};
		if let Some(action) = action {
			co.push(&context.local_identity(), CO_CORE_CO, &action).await?;
		}
	}
	Ok(vec![])
}
