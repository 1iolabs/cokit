use crate::{
	library::{invite_networks::invite_networks, is_cid_encrypted::is_cid_encrypted, join::create_join_message_from},
	services::application::action::{CoDidCommSendAction, NotifyAction},
	state::{query_core, Query},
	Action, CoContext, CoStorage, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use co_actor::Actions;
use co_core_membership::{Membership, MembershipState, Memberships, MembershipsAction};
use co_identity::{Identity, PrivateIdentityResolver};
use co_primitives::{CoId, CoInviteMetadata, Did, KnownTags};
use co_storage::BlockStorageExt;
use futures::{stream, FutureExt, Stream, StreamExt};

/// When a membership is set to active, try to connect the CO and send the join message via didcomm.
/// TODO: consensus finalization?
pub fn join_send(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	// filter
	let result = match action {
		Action::CoreAction { co, storage, context: _, action, cid: _ }
			if co.as_str() == CO_ID_LOCAL && action.core == CO_CORE_NAME_MEMBERSHIP =>
		{
			let mambership_action: MembershipsAction = action.get_payload().ok()?;
			match mambership_action {
				MembershipsAction::Join(membership) if membership.membership_state == MembershipState::Join => {
					Some((context.clone(), storage.clone(), membership.id, membership.did))
				},
				MembershipsAction::ChangeMembershipState { id, did, membership_state: MembershipState::Join } => {
					Some((context.clone(), storage.clone(), id, did))
				},
				_ => None,
			}
		},
		_ => None,
	};

	// join
	if let Some((context, storage, id, did)) = result {
		Some(
			async move { join_with_result(context.clone(), storage.clone(), id, did).await }
				.into_stream()
				.flat_map(Action::map_error_stream)
				.map(Ok),
		)
	} else {
		None
	}
}

/// Handle join when message sent succeeded.
///
/// In: [`Action::CoDidCommSent`]
/// Out: [`Action::JoinKeyRequest`] | [`Action::Joined`]
pub fn join_sent(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	_context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::CoDidCommSent {
			message:
				CoDidCommSendAction { co, notification: Some(NotifyAction::JoinSent { participant, encrypted }), .. },
			result: Ok(peers),
		} => {
			if let Some(peer) = peers.first() {
				let result = if *encrypted {
					Action::JoinKeyRequest { co: co.clone(), participant: participant.clone(), peer: *peer }
				} else {
					Action::Joined {
						co: co.clone(),
						participant: participant.clone(),
						success: true,
						peer: Some(*peer),
					}
				};
				Some(stream::iter([Ok(result)]))
			} else {
				None
			}
		},
		_ => None,
	}
}

async fn join_with_result(
	context: CoContext,
	storage: CoStorage,
	id: CoId,
	did: Did,
) -> Result<Vec<Action>, anyhow::Error> {
	if let Some(membership) = find_membership(&context, &storage, &id, &did).await? {
		Ok(vec![create_join_action(context, storage, membership).await?])
	} else {
		Ok(vec![])
	}
}

async fn find_membership(
	context: &CoContext,
	storage: &CoStorage,
	id: &CoId,
	did: &Did,
) -> anyhow::Result<Option<Membership>> {
	let local = context.local_co_reducer().await?;
	let memberships = query_core::<Memberships>(CO_CORE_NAME_MEMBERSHIP)
		.execute(storage, local.reducer_state().await.co())
		.await?;
	Ok(memberships.memberships.into_iter().find(|membership| {
		&membership.id == id && &membership.did == did
		// // we only handle remote invites
		// //  a join action is also used when an co is created
		// && membership.tags.find_key(&KnownTags::CoInviteMetadata.to_string()).is_some()
	}))
}

/// Create co join message action.
async fn create_join_action(context: CoContext, storage: CoStorage, membership: Membership) -> anyhow::Result<Action> {
	// metdata
	let invite_cid = membership
		.tags
		.link(&KnownTags::CoInviteMetadata.to_string())
		.ok_or(anyhow!("No co-invite-metadata"))?;
	let invite: CoInviteMetadata = storage.get_deserialized(invite_cid).await?;

	// message
	let private_identity_resolver = context.private_identity_resolver().await?;
	let identity = private_identity_resolver.resolve_private(&membership.did).await?;
	let (message_id, message) = create_join_message_from(&identity, membership.id.clone(), Some(invite.id.clone()))?;

	// send message to discovered peers until one send succedded and return Action::Joined.
	// this will also use invite.peer if possible.
	let networks = invite_networks(&context, &invite).await?;

	// result
	Ok(Action::CoDidCommSend(CoDidCommSendAction {
		co: membership.id.clone(),
		message,
		message_from: identity.identity().to_owned(),
		message_id,
		networks,
		notification: Some(NotifyAction::JoinSent {
			participant: membership.did.clone(),
			encrypted: is_membership_heads_encrypted(&storage, &membership).await?,
		}),
	}))
}

async fn is_membership_heads_encrypted(storage: &CoStorage, membership: &Membership) -> Result<bool, anyhow::Error> {
	for co_state in membership.state.iter() {
		let (_state, heads) = storage.get_value(&co_state.state).await?.into_value();
		return Ok(is_cid_encrypted(&heads));
	}
	Ok(false)
}
