use crate::{
	reducer::core_resolver::dynamic::DynamicCoreResolver, CoReducer, CoStorage, Reducer, ReducerChangeContext,
	ReducerChangedHandler,
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
			let mapping = match &self.encrypted_storage {
				Some(storage) => storage.flush_mapping().await?,
				None => None,
			};

			// next last heads
			let mut last_heads = reducer.heads().clone();
			swap(&mut self.last_heads, &mut last_heads);

			// update
			self.parent
				.push(
					&self.identity,
					&self.membership_core_name,
					&MembershipsAction::Update {
						id: self.id.to_owned(),
						state: *state,
						heads: reducer.heads().clone(),
						encryption_mapping: mapping,
						remove: last_heads,
					},
				)
				.await?;
		}
		Ok(())
	}
}
