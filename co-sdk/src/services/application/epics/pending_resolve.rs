// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	is_cid_encrypted,
	library::invite_networks::invite_networks,
	services::application::action::{CoDidCommSendAction, KeyRequestAction},
	state::{query_core, Query},
	Action, CoContext, CoReducerState, CoStorage, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use co_actor::{ActionDispatch, Actions};
use co_core_membership::{MembershipState, MembershipsAction};
use co_identity::{Identity, PrivateIdentityResolver};
use co_network::{EncodedMessage, HeadsMessage};
use co_primitives::{BlockStorageExt, CoId, CoInviteMetadata, Did, KnownTags, WeakCid};
use futures::Stream;
use std::collections::BTreeSet;

/// When a membership is set to Pending, resolve CO state (and optionally encryption key)
/// from the network, store the state in the membership, and transition to Active.
pub fn pending_resolve(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	// filter: CoreAction on local CO membership core with Pending state
	let result = match action {
		Action::CoreAction { co, storage, context: _, action, cid: _, head: _ }
			if co.as_str() == CO_ID_LOCAL && CO_CORE_NAME_MEMBERSHIP == action.core =>
		{
			let membership_action: MembershipsAction = action.get_payload().ok()?;
			match membership_action {
				MembershipsAction::Join(membership) if membership.membership_state == MembershipState::Pending => {
					Some((context.clone(), storage.clone(), actions.clone(), membership.id, membership.did))
				},
				MembershipsAction::ChangeMembershipState { id, did, membership_state: MembershipState::Pending } => {
					Some((context.clone(), storage.clone(), actions.clone(), id, did))
				},
				_ => None,
			}
		},
		_ => None,
	};

	// resolve
	let (context, storage, actions, co_id, did) = result?;
	Some(handle_pending_resolve(context, storage, actions, co_id, did))
}

fn handle_pending_resolve(
	context: CoContext,
	storage: CoStorage,
	actions: Actions<Action, (), CoContext>,
	co_id: CoId,
	did: Did,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	ActionDispatch::execute(actions, context.tasks(), {
		let co_id = co_id.clone();
		let did = did.clone();
		move |dispatch| async move {
			// read membership from local CO
			let local = context.local_co_reducer().await?;
			let memberships = query_core(CO_CORE_NAME_MEMBERSHIP)
				.execute(&storage, local.reducer_state().await.co())
				.await?;
			let membership = memberships
				.memberships
				.into_iter()
				.find(|m| m.id == co_id && m.did == did)
				.ok_or_else(|| anyhow!("Membership not found: {co_id}"))?;

			// read CoInviteMetadata from tags
			let invite_cid = membership
				.tags
				.link(&KnownTags::CoInviteMetadata.to_string())
				.ok_or(anyhow!("No co-invite-metadata tag on Pending membership"))?;
			let invite: CoInviteMetadata = storage.get_deserialized(invite_cid).await?;

			// build networks
			let networks = invite_networks(&context, &invite).await?;

			// resolve identity
			let private_identity_resolver = context.private_identity_resolver().await?;
			let identity = private_identity_resolver.resolve_private(&membership.did).await?;

			// send HeadsMessage::StateRequest
			let body = HeadsMessage::StateRequest(co_id.clone());
			let header = HeadsMessage::create_header(context.date());
			let (message_header, message) = EncodedMessage::create_signed_json(&identity, header, &body)?;
			let message_id = message_header.id.clone();

			let (state, heads) = dispatch
				.request(
					Action::CoDidCommSend(CoDidCommSendAction {
						co: co_id.clone(),
						networks,
						notification: None,
						tags: Default::default(),
						message_from: identity.identity().to_string(),
						message_header,
						message,
					}),
					move |action| filter_state_response(&message_id, &co_id, action),
				)
				.await?;

			// check encryption → request key if needed
			let reducer_state = CoReducerState::new_weak(Some(state), heads);
			if is_cid_encrypted(reducer_state.iter()) {
				let key_request = KeyRequestAction {
					co: membership.id.clone(),
					parent_co: CoId::from(CO_ID_LOCAL),
					key: None,
					from: Some(membership.did.clone()),
					network: None,
				};
				let request_clone = key_request.clone();
				dispatch
					.request(Action::KeyRequest(key_request), move |action| match action {
						Action::KeyRequestComplete(req, result) if req == &request_clone => Some(result.clone()),
						_ => None,
					})
					.await?
					.map_err(|e| anyhow!("Key request failed: {e}"))?;
			}

			// store state in membership via local CO reducer
			let local_co = context.local_co_reducer().await?;
			let co_state = reducer_state
				.to_external_co_state(&local_co.storage())
				.await?
				.ok_or_else(|| anyhow!("Expected state after resolve"))?;

			local_co
				.push(
					&identity,
					CO_CORE_NAME_MEMBERSHIP,
					&MembershipsAction::Update {
						id: membership.id.clone(),
						state: co_state,
						remove: Default::default(),
					},
				)
				.await?;

			// transition to Active
			local_co
				.push(
					&identity,
					CO_CORE_NAME_MEMBERSHIP,
					&MembershipsAction::ChangeMembershipState {
						id: membership.id.clone(),
						did: membership.did.clone(),
						membership_state: MembershipState::Active,
					},
				)
				.await?;

			// result
			Ok(())
		}
	})
}

fn filter_state_response(message_id: &str, co_id: &CoId, action: &Action) -> Option<(WeakCid, BTreeSet<WeakCid>)> {
	match action {
		Action::DidCommReceive { peer: _, message } => {
			if message.header().message_type == HeadsMessage::message_type()
				&& message.header().thid.as_deref() == Some(message_id)
			{
				let heads_message: HeadsMessage = message.body_deserialize().ok()?;
				match heads_message {
					HeadsMessage::State(received_co, state, heads) if &received_co == co_id => Some((state, heads)),
					_ => None,
				}
			} else {
				None
			}
		},
		_ => None,
	}
}
