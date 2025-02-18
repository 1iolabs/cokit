use super::{CoreResolver, CoreResolverError};
use crate::{CoReducer, ReducerChangeContext};
use async_trait::async_trait;
use cid::Cid;
use co_core_storage::StorageAction;
use co_identity::PrivateIdentity;
use co_primitives::{block_diff, BlockDiff, StoreParams};
use co_runtime::{RuntimeContext, RuntimePool};
use co_storage::BlockStorage;
use futures::{pin_mut, TryStreamExt};
use std::{marker::PhantomData, mem::swap};

/// Reference count state in a [`co_core_storage::Storage`] core.
pub struct ReferenceCoreResolver<S, C, P> {
	pinning_key: Option<String>,
	next: C,
	identity: P,
	parent: CoReducer,
	storage_core_name: String,
	_storage: PhantomData<S>,
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
			// diff
			let diff = block_diff(storage.clone(), *state, next_state, Default::default(), Default::default());

			// calc max references per action
			let max_references = <S::StoreParams as StoreParams>::MAX_BLOCK_SIZE / 2 / Cid::default().encoded_len();

			// apply references
			let mut references = Vec::new();
			let mut unreferences = Vec::new();
			pin_mut!(diff);
			while let Some(next) = diff.try_next().await? {
				// record
				match next {
					BlockDiff::Added(cid) => {
						references.push(cid);
					},
					BlockDiff::Removed(cid) => {
						unreferences.push(cid);
					},
				}

				// flush when we hit max block size
				if references.len() > max_references {
					let mut next_references = Vec::new();
					swap(&mut references, &mut next_references);
					let action = StorageAction::Reference(next_references);
					self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
				}
				if unreferences.len() > max_references {
					let mut next_unreferences = Vec::new();
					swap(&mut unreferences, &mut next_unreferences);
					let action = StorageAction::Unreference(next_unreferences);
					self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
				}
			}
			if !references.is_empty() {
				let action = StorageAction::Reference(references);
				self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
			}
			if !unreferences.is_empty() {
				let action = StorageAction::Unreference(unreferences);
				self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
			}

			// apply pin
			if let Some(pinning_key) = &self.pinning_key {
				let action = StorageAction::PinReference(pinning_key.clone(), vec![next_state]);
				self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
			}
		}

		// result
		Ok(next)
	}
}
