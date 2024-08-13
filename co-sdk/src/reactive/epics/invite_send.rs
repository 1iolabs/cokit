use crate::{
	drivers::network::CoNetworkTaskSpawner,
	library::{
		connect_and_send::connect_and_send,
		invite::{create_invite_message, CoInvitePayload},
		network_discovery::network_discovery,
		settings_timeout::settings_timeout,
	},
	reactive::context::{ActionObservable, StateObservable},
	state, Action, CoContext, CoNetwork, CoReducerFactory, CoStorage, KnownTag, CO_CORE_NAME_CO,
};
use anyhow::anyhow;
use co_core_co::{Co, CoAction};
use co_identity::{IdentityResolver, PrivateIdentityResolver};
use co_network::{didcomm::EncodedMessage, discovery::Discovery};
use co_primitives::{CoConnectivity, CoId, Did};
use futures::{Stream, StreamExt, TryStreamExt};
use std::{collections::BTreeSet, iter::empty};

/// When a participant is invited into an CO, try to connect and send the invite message via didcomm.
/// TODO: consensus finalization?
/// TODO: validate state? - action could have no effect in reducer (when already active, ...)
pub fn invite_send(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| async move {
			match action {
				Action::Invite { co, from, to } => Some((co, from, to)),
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
		.flat_map(move |(context, network, (co, from, participant))| {
			invite_discovery(context, network, co, from, participant)
		})
		.map(Action::map_error)
}

/// Dispatch Invite when a participant is invited into an CO.
pub fn invite_send_action(
	actions: ActionObservable,
	_states: StateObservable,
	_context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| async move {
			match action {
				Action::CoreAction { co, context, action, cid: _ }
					if context.is_local_change() && action.core == CO_CORE_NAME_CO =>
				{
					let co_action: CoAction = action.get_payload().ok()?;
					match co_action {
						CoAction::ParticipantInvite { participant, tags: _ } => Some((co, action.from, participant)),
						_ => None,
					}
				},
				_ => None,
			}
		})
		.map(|(co, from, to)| Action::Invite { co, from, to })
}

fn invite_discovery(
	context: CoContext,
	network: CoNetworkTaskSpawner,
	co: CoId,
	from: Did,
	to: Did,
) -> impl Stream<Item = anyhow::Result<Action>> + Send + 'static {
	async_stream::try_stream! {
		let timeout = settings_timeout(&context, &co, Some("invite")).await;
		let (message, discovery) = invite(&context, &co, &from, &to).await?;
		for await peer in connect_and_send(network, message, discovery, timeout) {
			if let Ok(peer) = peer {
				yield Action::InviteSent { co: co.clone(), participant: to.clone(), peer };
			}
		}
	}
}

async fn invite(
	context: &CoContext,
	co_id: &CoId,
	from: &Did,
	to: &Did,
) -> anyhow::Result<(EncodedMessage, BTreeSet<Discovery>)> {
	let identity_resolver = context.identity_resolver().await?;
	let co_reducer = context.co_reducer(co_id).await?.ok_or(anyhow!("Co not found: {}", co_id))?;
	let co = co_reducer.co().await?;
	let (state, heads) = co_reducer.external_reducer_state().await;
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
			connectivity: connectivity(co_reducer.storage(), &co).await?,
		},
		None,
	)?;

	// discovery
	let discovery = network_discovery(Some(&identity_resolver), &from_identity, None, empty(), [to.to_owned()])
		.try_collect()
		.await?;

	// result
	Ok((invite_message, discovery))
}

async fn connectivity(storage: CoStorage, co: &Co) -> anyhow::Result<CoConnectivity> {
	if !co.network.is_empty() {
		Ok(CoConnectivity {
			network: state::stream(storage, &co.network).try_collect().await?,
			participants: Default::default(),
		})
	} else {
		Ok(CoConnectivity {
			network: Default::default(),
			participants: co
				.participants
				.iter()
				.filter(|(_, participant)| {
					let network = CoNetwork::from_tags(&participant.tags).unwrap_or_default();
					network.has_feature(CoNetwork::Invite)
				})
				.map(|(participant, _)| participant.to_owned())
				.collect(),
		})
	}
}
