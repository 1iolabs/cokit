use crate::{
	library::invite::{CoInvitePayload, CO_DIDCOMM_INVITE},
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CoInvite, CO_CORE_MEMBERSHIP,
};
use anyhow::anyhow;
use co_core_membership::{Membership, MembershipState, MembershipsAction};
use co_identity::DidCommHeader;
use co_primitives::{tags, Did};
use futures::{future::ready, Stream, StreamExt};
use libp2p::PeerId;

/// When we receive a invite message:
/// - decide if want to be invited
/// - write membership
/// - send join
///
/// TODO: consensus validation
pub fn invite_receive(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| {
			ready(match action {
				Action::DidCommReceive { peer, message } => {
					if &message.header().message_type == CO_DIDCOMM_INVITE
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
			async move { invited(context, peer, header, body).await }
		})
		.flat_map(Action::map_error_stream)
}

async fn invited(context: CoContext, peer: PeerId, header: DidCommHeader, body: String) -> anyhow::Result<Vec<Action>> {
	let payload: CoInvitePayload = serde_json::from_str(&body)?;
	let local = context.local_co_reducer().await?;
	let co = local.co().await?;
	let invite = CoInvite::from_tags(&co.tags).unwrap_or_default();
	let did = header.to.first().ok_or(anyhow!("invalid header: to"))?.to_string();

	// state
	let membership_state = match invite {
		CoInvite::Manual => Some(MembershipState::Invite),
		CoInvite::Disable => None,
		CoInvite::All => Some(MembershipState::Active),
		CoInvite::Did => {
			todo!()
		},
	};

	// apply
	if let Some(membership_state) = membership_state {
		// membership
		local
			.push(
				&context.local_identity(),
				CO_CORE_MEMBERSHIP,
				&MembershipsAction::Join(membership(did, payload, membership_state)),
			)
			.await?;
	}
	Ok(vec![])
}

fn membership(did: Did, payload: CoInvitePayload, membership_state: MembershipState) -> Membership {
	Membership {
		id: payload.id,
		did,
		state: payload.state,
		heads: payload.heads,
		encryption_mapping: None,
		key: None,
		membership_state,
		tags: tags!(),
	}
}
