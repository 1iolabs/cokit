// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::{create_reducer_action::create_reducer_action, membership_all_heads::membership_all_heads},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	CoReducer, CoReducerState, CoStorage, Reducer, ReducerChangeContext, ReducerChangedHandler,
};
use async_trait::async_trait;
use co_core_membership::{CoState, MembershipsAction};
use co_identity::PrivateIdentityBox;
use co_primitives::{CoId, StaticCoDate, WeakCid};
use co_storage::EncryptedBlockStorage;
use std::collections::BTreeSet;

/// Apply reducer state/head changes to the membership core in the parent CO.
pub struct MembershipWriter {
	/// The membership CO UUID.
	pub id: CoId,
	/// The membership DID.
	// did: Did,
	pub parent: CoReducer,
	pub membership_core_name: String,
	pub identity: PrivateIdentityBox,
	pub encrypted_storage: Option<EncryptedBlockStorage<CoStorage>>,
	pub last_state: BTreeSet<CoState>,
}
impl MembershipWriter {
	pub fn new(
		id: CoId,
		parent: CoReducer,
		membership_core_name: String,
		identity: PrivateIdentityBox,
		encrypted_storage: Option<EncryptedBlockStorage<CoStorage>>,
		last_state: BTreeSet<CoState>,
	) -> Self {
		Self { id, parent, membership_core_name, identity, encrypted_storage, last_state }
	}

	pub async fn write(&mut self, storage: &CoStorage, reducer_state: CoReducerState) -> Result<(), anyhow::Error> {
		// action
		let parent_storage = self.parent.storage();
		if let Some((state, mappings)) = reducer_state.to_co_state(&parent_storage, storage).await? {
			// log
			tracing::trace!(?reducer_state, co = ?self.id, "membership-write");

			// apply mappings
			if let Some(mappings) = mappings {
				// make sure the root mappings are available in root storage
				// TODO: move this into the reducer? we only need to keep alive what the reducers know (snapshots?)?
				if let Some(encrypted_storage) = &self.encrypted_storage {
					encrypted_storage.insert_mappings(mappings).await;
				}
			}

			// get last heads to remove
			let remove = membership_all_heads(&parent_storage, self.last_state.iter())
				.await?
				.into_iter()
				.map(WeakCid::from)
				.collect();

			// apply to parent
			self.parent
				.push_reference(
					&self.identity,
					create_reducer_action(
						&parent_storage,
						&self.identity,
						&self.membership_core_name,
						&MembershipsAction::Update { id: self.id.to_owned(), state: state.clone(), remove },
						Default::default(),
						&StaticCoDate(0),
					)
					.await?,
				)
				.await?;

			// apply to self
			self.last_state = [state].into_iter().collect();
		}
		Ok(())
	}
}
#[async_trait]
impl ReducerChangedHandler<CoStorage, DynamicCoreResolver<CoStorage>> for MembershipWriter {
	async fn on_state_changed(
		&mut self,
		storage: &CoStorage,
		reducer: &Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		self.write(storage, CoReducerState::new_reducer(reducer)).await
	}
}
