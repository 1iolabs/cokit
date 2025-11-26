use crate::{
	find_membership,
	library::invite::{CoInvitePayload, CO_DIDCOMM_INVITE},
	Action, CoContext, CoInvite, CoReducerState, KnownTag, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use co_actor::Actions;
use co_core_membership::{Membership, MembershipState, MembershipsAction};
use co_identity::DidCommHeader;
use co_network::PeerId;
use co_primitives::{from_json_string, tags, CoInviteMetadata, Did, KnownTags, Tags};
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use futures::{FutureExt, Stream, StreamExt};
use std::collections::BTreeSet;

/// When we receive a invite message:
/// - decide if want to be invited
/// - write membership
/// - send join
///
/// TODO: consensus validation
pub fn invite_receive(
	_actions: &Actions<Action, (), CoContext>,
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
				let context = context.clone();
				let peer = *peer;
				Some(
					async move { invited(context, peer, header, body).await }
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

async fn invited(context: CoContext, peer: PeerId, header: DidCommHeader, body: String) -> anyhow::Result<Vec<Action>> {
	let payload: CoInvitePayload = from_json_string(&body)?;
	let local = context.local_co_reducer().await?;
	let (storage, co) = local.co().await?;
	let invite = CoInvite::from_tags(&co.tags).unwrap_or_default();
	let from = header.from.ok_or(anyhow!("invalid header: from"))?.to_string();
	let did = header.to.first().ok_or(anyhow!("invalid header: to"))?.to_string();

	// already exists?
	if find_membership(&local, &payload.id).await?.is_some() {
		return Ok(vec![]);
	}

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
		// storage
		#[cfg(feature = "pinning")]
		{
			local
				.push(
					&context.local_identity(),
					crate::CO_CORE_NAME_STORAGE,
					&co_core_storage::StorageAction::PinCreate(
						crate::types::co_pinning_key::CoPinningKey::State.to_string(&payload.id),
						context.settings().setting_co_default_max_state(),
						vec![payload.state.into()],
					),
				)
				.await?;
			local
				.push(
					&context.local_identity(),
					crate::CO_CORE_NAME_STORAGE,
					&co_core_storage::StorageAction::PinCreate(
						crate::types::co_pinning_key::CoPinningKey::Log.to_string(&payload.id),
						context.settings().setting_co_default_max_log(),
						payload.heads.iter().map(Into::into).collect(),
					),
				)
				.await?;
		}

		// payload
		let metadata = CoInviteMetadata {
			id: header.id,
			from,
			network: payload.connectivity.clone(),
			peer: Some(peer.to_bytes()),
		};
		let membership_tags = tags!(
			{KnownTags::CoInviteMetadata}: storage.set_serialized(&metadata).await?,
		);

		// membership
		local
			.push(
				&context.local_identity(),
				CO_CORE_NAME_MEMBERSHIP,
				&MembershipsAction::Join(membership(&storage, did, payload, membership_state, membership_tags).await?),
			)
			.await?;
	}
	Ok(vec![])
}

async fn membership(
	storage: &impl BlockStorage,
	did: Did,
	payload: CoInvitePayload,
	membership_state: MembershipState,
	membership_tags: Tags,
) -> Result<Membership, StorageError> {
	let reducer_state = CoReducerState::new(Some(payload.state), payload.heads.clone());
	let co_state = reducer_state.to_external_co_state(storage).await?.unwrap();
	Ok(Membership {
		id: payload.id,
		did,
		state: BTreeSet::from([co_state]),
		key: None,
		membership_state,
		tags: membership_tags,
	})
}
