use crate::{
	library::to_external_cid::{to_external_cid, to_external_cids},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	CoReducer, CoStorage, Reducer, ReducerChangeContext, ReducerChangedHandler,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_membership::MembershipsAction;
use co_identity::PrivateIdentity;
use co_primitives::CoId;
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
	I: PrivateIdentity + Debug + Send + Sync,
{
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		if let Some(state) = reducer.state() {
			// next
			let mut next_state = *state;
			let mut next_heads = reducer.heads().clone();
			if let Some(encrypted_storage) = &self.encrypted_storage {
				let mapping = encrypted_storage.content_mapping();
				next_state = to_external_cid(&mapping, next_state).await;
				next_heads = to_external_cids(&mapping, next_heads).await;
			}

			// next last heads
			let mut last_heads = next_heads.clone();
			swap(&mut self.last_heads, &mut last_heads);

			// update
			self.parent
				.push(
					&self.identity,
					&self.membership_core_name,
					&MembershipsAction::Update {
						id: self.id.to_owned(),
						state: next_state.into(),
						heads: next_heads.into_iter().map(Into::into).collect(),
						encryption_mapping: None,
						remove: last_heads.into_iter().map(Into::into).collect(),
					},
				)
				.await?;
		}
		Ok(())
	}
}
