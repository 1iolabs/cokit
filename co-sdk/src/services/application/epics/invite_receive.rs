use crate::{
	application::co_context::CoPinningKey,
	library::invite::{CoInvitePayload, CO_DIDCOMM_INVITE},
	Action, CoContext, CoInvite, KnownTag, CO_CORE_NAME_MEMBERSHIP, CO_CORE_NAME_STORAGE,
};
use anyhow::anyhow;
use co_core_membership::{CoState, Membership, MembershipState, MembershipsAction};
use co_core_storage::StorageAction;
use co_identity::DidCommHeader;
use co_primitives::{from_json_string, tags, CoInviteMetadata, Did, KnownTags, Tags};
use co_storage::BlockStorageExt;
use futures::{future::ready, stream, Stream, StreamExt};
use libp2p::PeerId;
use std::collections::BTreeSet;

/// When we receive a invite message:
/// - decide if want to be invited
/// - write membership
/// - send join
///
/// TODO: consensus validation
pub fn invite_receive(
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidCommReceive { peer, message } => {
			if &message.header().message_type == CO_DIDCOMM_INVITE
				&& message.header().to.len() == 1
				&& message.is_validated_sender()
			{
				let (header, body) = message.clone().into_inner();
				Some(
					stream::once(ready((context.clone(), *peer, header, body)))
						.then(
							move |(context, peer, header, body)| async move { invited(context, peer, header, body).await },
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

async fn invited(context: CoContext, peer: PeerId, header: DidCommHeader, body: String) -> anyhow::Result<Vec<Action>> {
	let payload: CoInvitePayload = from_json_string(&body)?;
	let local = context.local_co_reducer().await?;
	let co = local.co().await?;
	let invite = CoInvite::from_tags(&co.tags).unwrap_or_default();
	let from = header.from.ok_or(anyhow!("invalid header: from"))?.to_string();
	let did = header.to.first().ok_or(anyhow!("invalid header: to"))?.to_string();

	// state
	let membership_state = match invite {
		CoInvite::Manual => Some(MembershipState::Invite),
		CoInvite::Disable => None,
		CoInvite::Accept => Some(MembershipState::Join),
		CoInvite::Did => {
			todo!()
		},
	};

	// apply
	if let Some(membership_state) = membership_state {
		// payload
		let metadata = CoInviteMetadata {
			id: header.id,
			from,
			network: payload.connectivity.clone(),
			peer: Some(peer.to_bytes()),
		};
		let membership_tags = tags!(
			{KnownTags::CoInviteMetadata}: local.storage().set_serialized(&metadata).await?,
		);

		// storage
		local
			.push(
				&context.local_identity(),
				CO_CORE_NAME_STORAGE,
				&StorageAction::PinCreate(
					CoPinningKey::State.to_string(&payload.id),
					context.settings().setting_co_default_max_state(),
					vec![payload.state],
				),
			)
			.await?;
		local
			.push(
				&context.local_identity(),
				CO_CORE_NAME_STORAGE,
				&StorageAction::PinCreate(
					CoPinningKey::Log.to_string(&payload.id),
					context.settings().setting_co_default_max_log(),
					payload.heads.iter().cloned().collect(),
				),
			)
			.await?;

		// membership
		local
			.push(
				&context.local_identity(),
				CO_CORE_NAME_MEMBERSHIP,
				&MembershipsAction::Join(membership(did, payload, membership_state, membership_tags)),
			)
			.await?;
	}
	Ok(vec![])
}

fn membership(
	did: Did,
	payload: CoInvitePayload,
	membership_state: MembershipState,
	membership_tags: Tags,
) -> Membership {
	Membership {
		id: payload.id,
		did,
		state: BTreeSet::from([CoState {
			state: payload.state.into(),                           // TODO: consensus validation
			heads: payload.heads.iter().map(Into::into).collect(), // TODO: consensus validation
			encryption_mapping: None,
		}]),
		key: None,
		membership_state,
		tags: membership_tags,
	}
}
