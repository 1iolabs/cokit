use crate::{
	library::storage_snapshots::storage_snapshots,
	reducer::state_resolver::{StateResolver, StateResolverContext},
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{AnyBlockStorage, CoId};
use co_storage::BlockStorageContentMapping;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use std::{collections::BTreeSet, fmt::Debug, marker::PhantomData};

/// Tries to resolve states from the current CO storage core.
pub struct LocalStorageStateResolver<S> {
	co: CoId,
	_s: PhantomData<S>,
}
impl<S> Debug for LocalStorageStateResolver<S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("StorageStateResolver").finish()
	}
}
impl<S> LocalStorageStateResolver<S> {
	pub fn new(co: CoId) -> Self {
		Self { co, _s: PhantomData }
	}
}
#[async_trait]
impl<S: AnyBlockStorage + BlockStorageContentMapping> StateResolver<S> for LocalStorageStateResolver<S> {
	async fn resolve_state(
		&self,
		_storage: &S,
		_context: &StateResolverContext,
		_heads: &BTreeSet<Cid>,
	) -> Result<Option<(Cid, BTreeSet<Cid>)>, anyhow::Error> {
		Ok(None)
	}

	fn provide_roots(
		&self,
		storage: &S,
		context: &StateResolverContext,
	) -> Option<BoxStream<'static, Result<(Cid, BTreeSet<Cid>), anyhow::Error>>> {
		// as snapshots are chronological just use the latest
		let states = storage_snapshots(storage.clone(), context.state.into(), &self.co, storage.clone())
			.try_filter_map(|reducer_state| async move { Ok(reducer_state.some()) })
			.take(1);
		Some(states.boxed())
	}

	async fn push_state(&mut self, _storage: &S, _context: &StateResolverContext) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
