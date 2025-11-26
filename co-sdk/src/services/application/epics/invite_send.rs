use crate::{
	library::invite::{create_invite_message, CoInvitePayload},
	services::application::action::{CoDidCommSendAction, NotifyAction},
	state, Action, CoContext, CoNetwork, CoReducerFactory, CoStorage, KnownTag, CO_CORE_NAME_CO,
};
use anyhow::anyhow;
use co_actor::Actions;
use co_core_co::{Co, CoAction};
use co_identity::{DidCommHeader, IdentityResolver, PrivateIdentityResolver};
use co_network::{identities_networks, EncodedMessage};
use co_primitives::{CoConnectivity, CoId, Did, Network};
use futures::{stream, FutureExt, Stream, TryStreamExt};
use std::{collections::BTreeSet, future::ready};

/// Dispatch Invite when a participant is invited into an CO.
/// In: [`Action::CoreAction`]
/// Out: [`Action::Invite`]
/// TODO: consensus finalization?
/// TODO: validate state? - action could have no effect in reducer (when already active, ...)
pub fn invite_send(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::CoreAction { co, storage: _, context: action_context, action, cid: _, head: _ }
			if action_context.is_local_change() && CO_CORE_NAME_CO == action.core =>
		{
			let co_action: CoAction = action.get_payload().ok()?;
			match co_action {
				CoAction::ParticipantInvite { participant, tags: _ } => {
					let co = co.clone();
					let context = context.clone();
					let from = action.from.clone();
					let to = participant.clone();
					Some(
						async move {
							let (message_header, message, networks) = create_invite(&context, &co, &from, &to).await?;
							Ok(Action::CoDidCommSend(CoDidCommSendAction {
								co,
								networks,
								notification: Some(NotifyAction::InviteSent { to }),
								message_from: from,
								message_header,
								message,
								tags: Default::default(),
							}))
						}
						.into_stream(),
					)
				},
				_ => None,
			}
		},
		_ => None,
	}
}

/// Dispatch InviteSent when message sent succeeded.
///
/// In: [`Action::CoDidCommSent`]
/// Out: [`Action::InviteSent`]
pub fn invite_sent(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	_context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::CoDidCommSent {
			message: CoDidCommSendAction { co, notification: Some(NotifyAction::InviteSent { to }), .. },
			result: Ok(peers),
		} => {
			if let Some(peer) = peers.first() {
				Some(stream::iter([Ok(Action::InviteSent { co: co.clone(), to: to.clone(), peer: *peer })]))
			} else {
				None
			}
		},
		_ => None,
	}
}

async fn create_invite(
	context: &CoContext,
	co_id: &CoId,
	from: &Did,
	to: &Did,
) -> anyhow::Result<(DidCommHeader, EncodedMessage, BTreeSet<Network>)> {
	let identity_resolver = context.identity_resolver().await?;
	let co_reducer = context.try_co_reducer(co_id).await?;
	let (storage, co) = co_reducer.co().await?;
	let (state, heads) = co_reducer.reducer_state().await.to_external(&storage).await.into();
	let from_identity = context.private_identity_resolver().await?.resolve_private(from).await?;
	let to_identity = context.identity_resolver().await?.resolve(to).await?;

	// message
	let (invite_message_header, invite_message) = create_invite_message(
		&from_identity,
		&to_identity,
		CoInvitePayload {
			id: co_id.to_owned(),
			tags: co.tags.clone(),
			state: state.ok_or(anyhow!("Can not invite to empty CO"))?,
			heads,
			connectivity: connectivity(storage, &co).await?,
		},
		None,
	)?;

	// networks
	let networks = identities_networks(Some(&identity_resolver), [to.clone()])
		.try_collect()
		.await?;

	// result
	Ok((invite_message_header, invite_message, networks))
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
				.stream(&storage)
				.try_filter(|(_, participant)| ready(participant.state.is_active()))
				.try_filter(|(_, participant)| {
					let network = CoNetwork::from_tags(&participant.tags).unwrap_or_default();
					ready(network.has_feature(CoNetwork::Invite))
				})
				.map_ok(|(participant, _)| participant.to_owned())
				.try_collect()
				.await?,
		})
	}
}
