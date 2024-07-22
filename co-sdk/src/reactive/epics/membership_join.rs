use crate::{
	drivers::network::{tasks::didcomm_send::DidCommSendNetworkTask, CoNetworkTaskSpawner},
	library::{
		connect_and_send::connect_and_send, join::create_join_message_from, network_discovery::network_discovery,
		settings_timeout::settings_timeout,
	},
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use co_core_membership::{Membership, MembershipState, Memberships, MembershipsAction};
use co_identity::{IdentityBox, IdentityResolver, PrivateIdentityResolver};
use co_primitives::{CoInviteMetadata, KnownTags};
use co_storage::BlockStorageExt;
use futures::{pin_mut, stream, Stream, StreamExt, TryStreamExt};
use libp2p::PeerId;
use std::{future::ready, time::Duration};

/// When a membership is set to active, try to connect the CO and send the join message via didcomm.
/// TODO: consensus finalization?
pub fn membership_join(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| async move {
			match action {
				Action::CoreAction { co, context: _, action, cid: _ }
					if co.as_str() == CO_ID_LOCAL && action.core == CO_CORE_NAME_MEMBERSHIP =>
				{
					let mambership_action: MembershipsAction = action.get_payload().ok()?;
					match mambership_action {
						MembershipsAction::Join(membership)
							if membership.membership_state == MembershipState::Active =>
						{
							Some((membership.id, membership.did))
						},
						MembershipsAction::ChangeMembershipState {
							id,
							did,
							membership_state: MembershipState::Active,
						} => Some((id, did)),
						_ => None,
					}
				},
				_ => None,
			}
		})
		.then({
			let context = context.clone();
			move |(id, did)| {
				let context = context.clone();
				async move {
					let local = context.local_co_reducer().await?;
					let memberships: Memberships = local.state(CO_CORE_NAME_MEMBERSHIP).await?;
					Ok(memberships
						.memberships
						.into_iter()
						.find(|membership| membership.id == id && membership.did == did))
				}
			}
		})
		.try_filter_map(|membership| ready(Ok(membership)))
		.try_filter_map({
			let context = context.clone();
			move |membership| {
				let context = context.clone();
				async move {
					let network = match context.network().await {
						Some(n) => n,
						None => return Ok(None),
					};
					join(context, network, membership).await?;
					Ok(None)
				}
			}
		})
		.map(Action::map_error::<anyhow::Error>)
}

async fn join(
	context: CoContext,
	network: CoNetworkTaskSpawner,
	membership: Membership,
) -> anyhow::Result<Vec<Action>> {
	let local_co = context.local_co_reducer().await?;
	let mut result = Vec::new();

	// timeout
	let timeout: Duration = settings_timeout(&context, &membership.id, Some("join")).await;

	// metdata
	let invite_cid = membership
		.tags
		.link(&KnownTags::CoInviteMetadata.to_string())
		.ok_or(anyhow!("No co-invite-metadata"))?;
	let invite: CoInviteMetadata = local_co.storage().get_deserialized(invite_cid).await?;
	let invite_peer = PeerId::from_bytes(&invite.peer)?;

	// message
	let identity_resolver = context.private_identity_resolver().await?;
	let identity = identity_resolver.resolve_private(&membership.did).await?;
	let message = create_join_message_from(&identity, membership.id.clone(), Some(invite.id.clone()))?;

	// try use active connection
	if DidCommSendNetworkTask::send(network.clone(), [invite_peer].into_iter().collect(), message.clone(), timeout)
		.await
		.is_ok()
	{
		return Ok(vec![Action::Joined {
			co: membership.id.clone(),
			participant: membership.did.clone(),
			peer: invite_peer,
		}]);
	}

	// use connectivity settings
	//  send message to discovered peers until one send succedded and return Action::Joined.
	let resolver = context.identity_resolver().await?;
	let participants: Vec<IdentityBox> = stream::iter(invite.network.participants)
		.filter_map(move |did| {
			let resolver = resolver.clone();
			async move {
				match resolver.resolve(&did).await {
					Ok(i) => Some(i),
					Err(err) => {
						tracing::warn!(?err, ?did, "resolve-identity-failed");
						None
					},
				}
			}
		})
		.collect()
		.await;
	let discovery = network_discovery(&identity, &membership.id, invite.network.network, participants).await?;
	let join = connect_and_send(network, message, discovery, timeout);
	pin_mut!(join);
	while let Some(item) = join.next().await {
		match item {
			Ok(peer) => {
				result.push(Action::Joined { co: membership.id.clone(), participant: membership.did.clone(), peer });
				break;
			},
			Err(err) => {
				tracing::warn!(?err, "join-send-failed");
			},
		}
	}

	// result
	Ok(result)
}
