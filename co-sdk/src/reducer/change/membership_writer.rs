use crate::{
	library::to_external_cid::{to_external_cid, to_external_cids_map},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	CoReducer, CoStorage, Reducer, ReducerChangeContext, ReducerChangedHandler,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_membership::MembershipsAction;
use co_identity::PrivateIdentity;
use co_primitives::{CoId, WeakCid};
use co_storage::EncryptedBlockStorage;
use std::{collections::BTreeSet, fmt::Debug, mem::swap};

/// Apply reducer state/head changes to the membership core in the parent CO.
pub struct MembershipWriter<I> {
	/// The membership CO UUID.
	pub id: CoId,
	/// The membership DID.
	// did: Did,
	pub parent: CoReducer,
	pub membership_core_name: String,
	pub identity: I,
	pub encrypted_storage: Option<EncryptedBlockStorage<CoStorage>>,
	pub last_heads: BTreeSet<Cid>,
}

impl<I> MembershipWriter<I> {
	pub fn new(
		id: CoId,
		parent: CoReducer,
		membership_core_name: String,
		identity: I,
		encrypted_storage: Option<EncryptedBlockStorage<CoStorage>>,
		last_heads: BTreeSet<Cid>,
	) -> Self {
		Self { id, parent, membership_core_name, identity, encrypted_storage, last_heads }
	}
}
#[async_trait]
impl<I> ReducerChangedHandler<CoStorage, DynamicCoreResolver<CoStorage>> for MembershipWriter<I>
where
	I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
{
	async fn on_state_changed(
		&mut self,
		storage: &CoStorage,
		reducer: &Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		if let Some(state) = reducer.state() {
			// next
			let next_state = to_external_cid(storage, *state).await;
			let next_heads_map = to_external_cids_map(storage, reducer.heads().clone()).await;

			// make sure the root mappings are available in parent storage
			if let Some(encrypted_storage) = &self.encrypted_storage {
				encrypted_storage
					.insert_mappings([(*state, next_state)].into_iter().chain(next_heads_map.clone()))
					.await;
			}

			// next last heads
			let mut last_heads: BTreeSet<Cid> = next_heads_map.values().cloned().collect();
			swap(&mut self.last_heads, &mut last_heads);

			// update
			self.parent
				.push(
					&self.identity,
					&self.membership_core_name,
					&MembershipsAction::Update {
						id: self.id.to_owned(),
						state: next_state.into(),
						heads: next_heads_map.values().map(WeakCid::from).collect(),
						encryption_mapping: None,
						remove: last_heads.into_iter().map(Into::into).collect(),
					},
				)
				.await?;
		}
		Ok(())
	}
}
