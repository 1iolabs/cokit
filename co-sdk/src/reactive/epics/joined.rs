use crate::{
	application::shared::SharedCoBuilder,
	reactive::context::{ActionObservable, StateObservable},
	state, Action, CoContext, CoStorage, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use co_core_co::Co;
use co_core_membership::{Membership, Memberships, MembershipsAction};
use co_identity::PrivateIdentityResolver;
use co_network::StaticPeerProvider;
use co_primitives::{CoId, Did, KnownMultiCodec, MultiCodec};
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use libipld::Cid;
use libp2p::PeerId;
use std::{collections::BTreeSet, future::ready};

/// Fetch co core state and set membership to active when joined or back to invite when failed.
/// TODO: validate consensus?
pub fn joined(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.clone()
		.filter_map(|action| {
			ready(match action {
				Action::Joined { co, participant, success, peer } => Some((co, participant, success, peer)),
				_ => None,
			})
		})
		.then(move |(id, did, success, peer)| {
			let context = context.clone();
			async move {
				// fetch
				if let Some(peer) = peer {
					joined_initialize(context, id.clone(), did.clone(), peer).await?;
				}

				// active
				let payload = MembershipsAction::ChangeMembershipState {
					id: id.clone(),
					did: did.clone(),
					membership_state: if success {
						co_core_membership::MembershipState::Active
					} else {
						co_core_membership::MembershipState::Invite
					},
				};
				Ok(Action::push(CO_ID_LOCAL, did, CO_CORE_NAME_MEMBERSHIP, payload))
			}
		})
		.map(Action::map_error::<anyhow::Error>)
}

/// Initialize the joined CO using the already connected peer.
///
/// We fetch at least the co state with networks and participants so we can reconnect later.
/// Also we resolve the state and heads to the actual CIS as the invite contains the encrypted versions.
#[tracing::instrument(err, skip(context))]
async fn joined_initialize(context: CoContext, id: CoId, did: Did, peer: PeerId) -> anyhow::Result<()> {
	let local_co = context.local_co_reducer().await?;
	let membership =
		find_membership(&context, &id, &did)
			.await?
			.ok_or(anyhow::anyhow!("Membership not found: {} ({})", id, did))?;
	// let co_reducer = context.co_reducer(&id).await?.ok_or(anyhow::anyhow!("Co not found: {}", id))?;
	let network = context.network().await.ok_or(anyhow::anyhow!("Expected network"))?;
	let builder = SharedCoBuilder::new(local_co, membership.clone());
	let secret = builder.secret().await?;
	let storage = builder.build_network_storage(
		StaticPeerProvider::new([peer].into_iter().collect()),
		network,
		secret.as_ref(),
		context.inner.storage(),
	)?;

	// let co_reducer = context
	// 	.inner
	// 	.create_co_instance_membership(local_co, membership.clone(), identity, None, false, true)
	// 	.await?;
	// let storage = NetworkBlockStorage::new(
	// 	co_reducer.storage(),
	// 	network,
	// 	StaticPeerProvider::new([peer].into_iter().collect()),
	// 	Duration::from_secs(30),
	// );

	// fetch state/heads
	stream::iter([&membership.state].into_iter().chain(membership.heads.iter()))
		.map(Result::<&Cid, StorageError>::Ok)
		.try_for_each_concurrent(None, |cid| {
			let storage = storage.clone();
			async move {
				storage.get(cid).await?;
				Ok(())
			}
		})
		.await?;

	// encrypted: update membership to enencrypted CIDs
	let storage = if let Some(secret) = &secret {
		let storage = builder.build_encrypted_storage(secret, storage).await?;
		let mut update_state: Option<Cid> = None;
		let mut update_heads: Option<BTreeSet<Cid>> = None;
		if MultiCodec::is(&membership.state, KnownMultiCodec::CoEncryptedBlock) {
			let plain = storage.get(&membership.state).await?;
			tracing::trace!(co = ?id, from = ?membership.state, to = ?plain.cid(), "joined-state-change");
			update_state = Some(*plain.cid());
		}
		if membership
			.heads
			.iter()
			.any(|head| MultiCodec::is(head, KnownMultiCodec::CoEncryptedBlock))
		{
			let plain = stream::iter(membership.heads.iter())
				.map(Result::<&Cid, StorageError>::Ok)
				.and_then(|head| {
					// let storage = storage.clone();
					async { Ok(*storage.get(head).await?.cid()) }
				})
				.try_collect()
				.await?;
			tracing::trace!(co = ?id, from = ?membership.heads, to = ?plain, "joined-heads-change");
			update_heads = Some(plain);
		}
		if update_state.is_some() || update_heads.is_some() {
			let local_co = context.local_co_reducer().await?;
			let identity = context.private_identity_resolver().await?.resolve_private(&did).await?;
			local_co
				.push(
					&identity,
					CO_CORE_NAME_MEMBERSHIP,
					&MembershipsAction::Update {
						id: membership.id,
						state: update_state.unwrap_or(membership.state),
						heads: update_heads.unwrap_or(membership.heads),
						encryption_mapping: membership.encryption_mapping,
					},
				)
				.await?;
		}
		CoStorage::new(storage)
	} else {
		storage
	};

	// fetch network settings and participants
	let co: Co = storage.get_deserialized(&membership.state).await?;
	state::stream(storage.clone(), &co.network).try_collect::<Vec<_>>().await?;
	// TODO: participants DAG (https://gitlab.1io.com/1io/co-sdk/-/issues/39)
	// state::stream(co_reducer.storage(), &co.participants).try_collect::<Vec<_>>().await?;

	// fetch
	Ok(())
}

async fn find_membership(context: &CoContext, id: &CoId, did: &Did) -> anyhow::Result<Option<Membership>> {
	let local = context.local_co_reducer().await?;
	let memberships: Memberships = local.state(CO_CORE_NAME_MEMBERSHIP).await?;
	Ok(memberships
		.memberships
		.into_iter()
		.find(|membership| &membership.id == id && &membership.did == did))
}
