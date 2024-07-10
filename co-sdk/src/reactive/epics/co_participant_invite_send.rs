use crate::{
	drivers::network::{
		tasks::{didcomm_send::DidCommSendNetworkTask, discovery_connect::DiscoveryConnectNetworkTask},
		CoNetworkTaskSpawner,
	},
	library::{
		identity_discovery::identity_discovery,
		invite::{create_invite_message, CoInvitePayload},
	},
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CoReducerFactory, CO_CORE_NAME_CO,
};
use anyhow::anyhow;
use co_core_co::CoAction;
use co_identity::{IdentityResolver, PrivateIdentityResolver};
use co_network::{didcomm::EncodedMessage, discovery::Discovery};
use co_primitives::{CoId, Did, Tags};
use futures::{Stream, StreamExt};
use std::{collections::BTreeSet, time::Duration};

/// When a participant is invited into an CO, try to connect and send the invite message via didcomm.
/// TODO: consensus finalization?
pub fn co_participant_invite_send(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| async move {
			match action {
				Action::CoreAction { co, context, action, cid: _ }
					if context.is_local_change() && action.core == CO_CORE_NAME_CO =>
				{
					let co_action: CoAction = action.get_payload().ok()?;
					match co_action {
						CoAction::ParticipantInvite { participant, tags } => Some((co, action.from, participant, tags)),
						_ => None,
					}
				},
				_ => None,
			}
		})
		.filter({
			let context = context.clone();
			move |(co, ..)| {
				let co = co.clone();
				let context = context.clone();
				async move { context.is_shared(&co).await }
			}
		})
		.filter_map(move |data| {
			let context = context.clone();
			async move { context.network().await.map(|network| (context, network, data)) }
		})
		.flat_map(move |(context, network, (co, from, participant, participant_tags))| {
			invite_discovery(context, network, co, from, participant, participant_tags)
		})
		.map(Action::map_error)
}

/// TODO: read timeout setting from tags
fn invite_discovery(
	context: CoContext,
	network: CoNetworkTaskSpawner,
	co: CoId,
	from: Did,
	to: Did,
	participant_tags: Tags,
) -> impl Stream<Item = anyhow::Result<Action>> + Send + 'static {
	async_stream::try_stream! {
		let (message, discovery) = invite(&context, &co, &from, &to, &participant_tags).await?;
		let connect_peers = DiscoveryConnectNetworkTask::connect_with_timeout(network.clone(), discovery, Duration::from_secs(10));
		for await peer in connect_peers {
			if let Ok(peer) = peer {
				let send = DidCommSendNetworkTask::send(
					network.clone(),
					[peer].into_iter().collect(),
					message.clone(),
					Duration::from_secs(10),
				).await;
				if let Ok(peer) = send {
					yield Action::Invited { co: co.clone(), participant: to.clone(), peer };
				}
			}
		}
	}
}

async fn invite(
	context: &CoContext,
	co_id: &CoId,
	from: &Did,
	to: &Did,
	_participant_tags: &Tags,
) -> anyhow::Result<(EncodedMessage, BTreeSet<Discovery>)> {
	let co_reducer = context.co_reducer(co_id).await?.ok_or(anyhow!("Co not found: {}", co_id))?;
	let co = co_reducer.co().await?;
	let (state, heads) = co_reducer.reducer_state().await;
	let from_identity = context.private_identity_resolver().await?.resolve_private(from).await?;
	let to_identity = context.identity_resolver().await?.resolve(to).await?;

	// message
	let invite_message = create_invite_message(
		&from_identity,
		&to_identity,
		CoInvitePayload {
			id: co_id.to_owned(),
			tags: co.tags.clone(),
			state: state.ok_or(anyhow!("Can not invite to empty CO"))?,
			heads,
		},
		None,
	)?;

	// discovery
	let discovery = identity_discovery(&from_identity, &to_identity)?;

	// result
	Ok((invite_message, discovery))
}
