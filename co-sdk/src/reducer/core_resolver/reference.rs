use super::{CoreResolver, CoreResolverError};
use crate::{CoReducer, ReducerChangeContext, CO_CORE_NAME_STORAGE};
use async_trait::async_trait;
use cid::Cid;
use co_core_storage::StorageAction;
use co_identity::PrivateIdentity;
use co_primitives::{block_diff_added_with_parent, StoreParams};
use co_runtime::{RuntimeContext, RuntimePool};
use co_storage::BlockStorage;
use futures::{pin_mut, TryStreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	marker::PhantomData,
	mem::swap,
};

/// Reference count state in a [`co_core_storage::Storage`] core.
pub struct ReferenceCoreResolver<S, C, P> {
	pinning_key: Option<String>,
	next: C,
	identity: P,
	parent: CoReducer,
	storage_core_name: String,
	_storage: PhantomData<S>,
}
impl<S, C, P> ReferenceCoreResolver<S, C, P> {
	pub fn new(next: C, parent: CoReducer, identity: P, pinning_key: Option<String>) -> Self {
		Self {
			pinning_key,
			next,
			identity,
			parent,
			storage_core_name: CO_CORE_NAME_STORAGE.to_owned(),
			_storage: Default::default(),
		}
	}

	pub fn with_storage_core_name(mut self, name: String) -> Self {
		self.storage_core_name = name;
		self
	}
}
#[async_trait]
impl<S, C, P> CoreResolver<S> for ReferenceCoreResolver<S, C, P>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	C: CoreResolver<S> + Send + Sync + 'static,
	P: PrivateIdentity + Send + Sync + 'static,
{
	#[tracing::instrument(skip(self, storage, runtime, state, action))]
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		// execute
		let next = self.next.execute(storage, runtime, context, state, action).await?;

		// references
		if let Some(next_state) = next.state {
			// calc max references per action
			let max_references = <S::StoreParams as StoreParams>::MAX_BLOCK_SIZE / 2 / Cid::default().encoded_len();

			// diff
			let diff = block_diff_added_with_parent(
				storage.clone(),
				*state,
				next_state,
				Default::default(),
				Default::default(),
			);

			// apply root reference
			if let Some(pinning_key) = &self.pinning_key {
				let action = StorageAction::PinReference(pinning_key.clone(), vec![next_state]);
				self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
			}

			// apply structural references
			let mut references = BTreeMap::<Cid, BTreeSet<Cid>>::new();
			let mut references_count = 0;
			pin_mut!(diff);
			while let Some((next_parent, next)) = diff.try_next().await? {
				if let Some(next_parent) = next_parent {
					// record
					references.entry(next_parent).or_default().insert(next);
					references_count += 1;

					// flush when we hit max block size
					if references_count > max_references {
						// take
						let mut next_references = Default::default();
						swap(&mut references, &mut next_references);
						references_count = 0;

						// apply
						let action = StorageAction::ReferenceStructure(next_references.into_iter().collect());
						self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
					}
				}
			}
			if !references.is_empty() {
				let action = StorageAction::ReferenceStructure(references.into_iter().collect());
				self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
			}
		}

		// result
		Ok(next)
	}
}
