use crate::{
	actor::ActorHandle,
	drivers::network::{tasks::didcomm_send::DidCommSendNetworkTask, CoNetworkTaskSpawner},
	library::{
		invite_networks::invite_networks, is_cid_encrypted::is_cid_encrypted, join::create_join_message_from,
		settings_timeout::settings_timeout,
	},
	plugins::connections::ConnectionMessage,
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use co_core_membership::{Membership, MembershipState, Memberships, MembershipsAction};
use co_identity::{Identity, PrivateIdentityResolver};
use co_primitives::{CoId, CoInviteMetadata, Did, KnownTags};
use co_storage::BlockStorageExt;
use futures::{pin_mut, stream, Stream, StreamExt};
use std::{future::ready, time::Duration};

/// When a membership is set to active, try to connect the CO and send the join message via didcomm.
/// TODO: consensus finalization?
pub fn join_send(
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
						MembershipsAction::Join(membership) if membership.membership_state == MembershipState::Join => {
							Some((membership.id, membership.did))
						},
						MembershipsAction::ChangeMembershipState {
							id,
							did,
							membership_state: MembershipState::Join,
						} => Some((id, did)),
						_ => None,
					}
				},
				_ => None,
			}
		})
		.flat_map(move |(id, did)| join_with_result(context.clone(), id, did))
}

fn join_with_result(context: CoContext, id: CoId, did: Did) -> impl Stream<Item = Action> + Send + 'static {
	stream::once(ready((id.clone(), did.clone())))
		.flat_map({
			let context = context.clone();
			move |(id, did)| {
				let context = context.clone();
				async_stream::try_stream! {
					if let Some(network) = context.network().await {
						if let Some(membership) = find_membership(&context, &id, &did).await? {
							for action in join(context, network, membership).await? {
								yield action;
							}
						}
					}
				}
			}
		})
		.map(Action::map_error::<anyhow::Error>)
		// augment result with Joined action if not encrypted
		.flat_map(move |action| {
			let joined = match &action {
				Action::Error { err: _ } => {
					Some(Action::Joined { co: id.clone(), participant: did.clone(), success: false, peer: None })
				},
				Action::JoinSent { co: _, heads, participant: _, peer } if !is_cid_encrypted(heads.iter()) => {
					Some(Action::Joined { co: id.clone(), participant: did.clone(), success: true, peer: Some(*peer) })
				},
				_ => None,
			};
			let mut result = vec![action];
			if let Some(joined) = joined {
				result.push(joined);
			}
			stream::iter(result)
		})
}

async fn find_membership(context: &CoContext, id: &CoId, did: &Did) -> anyhow::Result<Option<Membership>> {
	let local = context.local_co_reducer().await?;
	let memberships: Memberships = local.state(CO_CORE_NAME_MEMBERSHIP).await?;
	Ok(memberships.memberships.into_iter().find(|membership| {
		&membership.id == id && &membership.did == did
		// // we only handle remote invites
		// //  a join action is also used when an co is created
		// && membership.tags.find_key(&KnownTags::CoInviteMetadata.to_string()).is_some()
	}))
}

async fn join(
	context: CoContext,
	(network, connections): (CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>),
	membership: Membership,
) -> anyhow::Result<Vec<Action>> {
	let local_co = context.local_co_reducer().await?;
	let mut result = Vec::new();

	// timeout
	let timeout: Duration = settings_timeout(&context, &CoId::from(CO_ID_LOCAL), Some("join")).await;

	// metdata
	let invite_cid = membership
		.tags
		.link(&KnownTags::CoInviteMetadata.to_string())
		.ok_or(anyhow!("No co-invite-metadata"))?;
	let invite: CoInviteMetadata = local_co.storage().get_deserialized(invite_cid).await?;

	// message
	let private_identity_resolver = context.private_identity_resolver().await?;
	let identity = private_identity_resolver.resolve_private(&membership.did).await?;
	let message = create_join_message_from(&identity, membership.id.clone(), Some(invite.id.clone()))?;

	// send message to discovered peers until one send succedded and return Action::Joined.
	// this will also use invite.peer if possible.
	let networks = invite_networks(&context, &invite).await?;
	let peers_stream =
		ConnectionMessage::co_use(connections, membership.id.clone(), identity.identity().to_string(), networks);
	pin_mut!(peers_stream);
	while let Some(peers) = peers_stream.next().await {
		match peers {
			Ok(peers) => {
				let send = DidCommSendNetworkTask::send(network.clone(), peers.added, message.clone(), timeout).await;
				match send {
					Ok(peer) => {
						result.push(Action::JoinSent {
							co: membership.id.clone(),
							participant: membership.did.clone(),
							heads: membership.heads.clone(),
							peer,
						});
						break;
					},
					Err(err) => {
						tracing::warn!(?err, "join-send-message-failed");
					},
				}
			},
			Err(err) => {
				tracing::warn!(?err, "join-send-failed");
			},
		}
	}

	// result
	Ok(result)
}
