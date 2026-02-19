// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	is_cid_encrypted,
	reducer::state_resolver::{StateResolver, StateResolverContext, StateStream},
	CoReducerState,
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::AnyBlockStorage;
use co_storage::BlockStorageContentMapping;
use futures::{stream, StreamExt};
use std::{collections::BTreeSet, fmt::Debug, future::ready, marker::PhantomData};

pub struct MembershipStateResolver<S> {
	snapshots: Vec<(bool, CoReducerState)>,
	_s: PhantomData<S>,
}
impl<S> Default for MembershipStateResolver<S> {
	fn default() -> Self {
		Self { snapshots: Default::default(), _s: Default::default() }
	}
}
impl<S> Debug for MembershipStateResolver<S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MembershipStateResolver")
			.field("snapshots", &self.snapshots)
			.finish()
	}
}
impl<S> MembershipStateResolver<S>
where
	S: AnyBlockStorage + BlockStorageContentMapping,
{
	/// Push new latest state.
	pub fn insert(&mut self, state: CoReducerState) {
		if is_cid_encrypted(state.iter()) {
			self.insert_extenral(state);
		} else {
			self.insert_internal(state);
		}
	}

	/// Push new latest state.
	pub fn insert_internal(&mut self, state: CoReducerState) {
		self.snapshots.push((false, state));
	}

	/// Push new latest state.
	pub fn insert_extenral(&mut self, state: CoReducerState) {
		self.snapshots.push((true, state));
	}

	/// Ensure all snapshots are internal.
	/// As this may caused network requests this is called on provide by the reducer.
	pub async fn ensure_internal(&mut self, storage: &S) -> Result<(), anyhow::Error> {
		for item in self.snapshots.iter_mut() {
			if item.0 {
				item.1 = item.1.to_internal(storage).await;
				item.0 = false;
			}
		}
		Ok(())
	}
}
#[async_trait]
impl<S> StateResolver<S> for MembershipStateResolver<S>
where
	S: AnyBlockStorage + BlockStorageContentMapping,
{
	async fn resolve_state(
		&self,
		_storage: &S,
		_context: &StateResolverContext,
		heads: &BTreeSet<Cid>,
	) -> Result<Option<(Cid, BTreeSet<Cid>)>, anyhow::Error> {
		for (external, reducer_state) in &self.snapshots {
			if !external && &reducer_state.1 == heads {
				if let Some(result) = reducer_state.some() {
					return Ok(Some(result));
				}
			}
		}
		Ok(None)
	}

	fn provide_roots(&mut self, _storage: &S, _context: &StateResolverContext) -> Option<StateStream> {
		Some(
			stream::iter(self.snapshots.clone())
				.filter_map(|(external, state)| ready(if !external { Some((state.0, state.1)) } else { None }))
				.map(Ok)
				.boxed(),
		)
	}

	/// Initialize membership state resolver by checking all snapshots are internal
	///
	/// # Note
	/// This will fetch the block from network if neccesarry.
	/// To prevent reducer init deadlocks we do this here to have the actor instance available for caller while doing
	/// the network stuff.
	async fn initialize(&mut self, storage: &S) -> Result<(), anyhow::Error> {
		self.ensure_internal(storage).await?;
		Ok(())
	}

	fn clear(&mut self) {
		self.snapshots.clear();
	}
}
