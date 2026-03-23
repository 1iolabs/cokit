// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	find_membership_by,
	library::{
		invite_networks::invite_networks, is_membership_heads_encrypted::is_membership_heads_encrypted,
		join::create_join_message_from,
	},
	services::application::action::{CoDidCommSendAction, NotifyAction},
	Action, CoContext, CoStorage, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use co_actor::Actions;
use co_core_membership::{Membership, MembershipsAction};
use co_identity::{Identity, PrivateIdentityResolver};
use co_primitives::{CoId, CoInviteMetadata, Did, KnownTags};
use co_storage::BlockStorageExt;
use futures::{FutureExt, Stream, StreamExt};
use std::future::ready;

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
		Action::CoreAction { co, storage, context: _, action, cid: _, head: _ }
			if co.as_str() == CO_ID_LOCAL && CO_CORE_NAME_MEMBERSHIP == action.core =>
		{
			let membership_action: MembershipsAction = action.get_payload().ok()?;
			match membership_action {
				MembershipsAction::JoinRequest { id, did, .. } => Some((context.clone(), storage.clone(), id, did)),
				MembershipsAction::InviteAccept { id, did, .. } => Some((context.clone(), storage.clone(), id, did)),
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
			message: CoDidCommSendAction { co, notification: Some(NotifyAction::JoinSent { participant, .. }), .. },
			result,
		} => {
			let peer = result.as_ref().ok().and_then(|result| result.first().cloned());
			let action =
				Action::Joined { co: co.clone(), participant: participant.clone(), success: peer.is_some(), peer };
			Some(ready(Ok(action)).into_stream())
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
	let local = context.local_co_reducer().await?;
	if let Some(membership) = find_membership_by(&local, id, Some(&did), None).await? {
		Ok(vec![create_join_action(context, storage, membership, did).await?])
	} else {
		Ok(vec![])
	}
}

/// Create co join message action.
async fn create_join_action(
	context: CoContext,
	storage: CoStorage,
	membership: Membership,
	did: Did,
) -> anyhow::Result<Action> {
	// metadata
	let invite_cid = membership
		.tags
		.link(&KnownTags::CoInviteMetadata.to_string())
		.ok_or(anyhow!("No co-invite-metadata"))?;
	let invite: CoInviteMetadata = storage.get_deserialized(invite_cid).await?;

	// message
	let private_identity_resolver = context.private_identity_resolver().await?;
	let identity = private_identity_resolver.resolve_private(&did).await?;
	let (message_header, message) =
		create_join_message_from(context.date(), &identity, membership.id.clone(), Some(invite.id.clone()))?;

	// send message to discovered peers until one send succeeded and return Action::Joined.
	// this will also use invite.peer if possible.
	let networks = invite_networks(&context, &invite).await?;

	// result
	Ok(Action::CoDidCommSend(CoDidCommSendAction {
		co: membership.id.clone(),
		message,
		message_from: identity.identity().to_owned(),
		message_header,
		networks,
		notification: Some(NotifyAction::JoinSent {
			participant: did,
			encrypted: is_membership_heads_encrypted(&storage, &membership).await?,
		}),
		tags: Default::default(),
	}))
}
