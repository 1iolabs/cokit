use crate::{
	library::sample_stream::sample_stream_ordered_first_last,
	reducer::state_resolver::{StateResolver, StateResolverContext},
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{AnyBlockStorage, SignedEntry};
use co_storage::{BlockStorageExt, StorageError};
use futures::{
	stream::{self, BoxStream},
	Stream, StreamExt, TryStreamExt,
};
use std::{collections::BTreeSet, fmt::Debug, marker::PhantomData, mem::take};

pub struct StaticStateResolver<S> {
	snapshots: Vec<(Cid, BTreeSet<Cid>)>,
	_s: PhantomData<S>,
}
impl<S> Default for StaticStateResolver<S> {
	fn default() -> Self {
		Self { snapshots: Default::default(), _s: Default::default() }
	}
}
impl<S> Debug for StaticStateResolver<S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("StaticStateResolver")
			.field("snapshots", &self.snapshots)
			.finish()
	}
}
impl<S: AnyBlockStorage> StaticStateResolver<S> {
	pub async fn new_unsorted(
		storage: &S,
		snapshots: impl Stream<Item = (Cid, BTreeSet<Cid>)>,
	) -> Result<Self, StorageError> {
		let snapshots = Self::sort(storage, snapshots).await?;
		Ok(Self { _s: PhantomData, snapshots })
	}

	// /// Insert unsorted state.
	// pub async fn insert(&mut self, storage: &S, state: Cid, heads: BTreeSet<Cid>) -> Result<(), StorageError> {
	// 	let clock = heads_clock(storage, &heads).await?;
	// 	self.snapshots.insert(clock, (state, heads));
	// }

	/// Push new latest state.
	pub fn push(&mut self, state: Cid, heads: BTreeSet<Cid>) {
		self.snapshots.push((state, heads));
	}

	pub fn is_empty(&self) -> bool {
		self.snapshots.is_empty()
	}

	pub fn len(&self) -> usize {
		self.snapshots.len()
	}

	pub async fn sort(
		storage: &S,
		snapshots: impl Stream<Item = (Cid, BTreeSet<Cid>)>,
	) -> Result<Vec<(Cid, BTreeSet<Cid>)>, anyhow::Error> {
		let mut unsorted_with_clock = snapshots
			.then(|(state, heads)| {
				let storage = storage.clone();
				async move {
					let clock = heads_clock(&storage, &heads).await?;
					Result::<_, StorageError>::Ok((clock, (state, heads)))
				}
			})
			.try_collect::<Vec<_>>()
			.await?;
		unsorted_with_clock.sort_by(|(a_time, _), (b_time, _)| a_time.cmp(b_time));
		Ok(unsorted_with_clock.into_iter().map(|(_, snapshot)| snapshot).collect())
	}

	/// Shrink snapshots by sample them, keeping first and last.
	pub async fn shrink(&mut self, k: usize) -> Result<(), anyhow::Error> {
		// already fits?
		if self.snapshots.len() <= k {
			return Ok(());
		}

		// shrink
		self.snapshots =
			sample_stream_ordered_first_last(stream::iter(take(&mut self.snapshots).into_iter().map(Ok)), k).await?;

		// result
		Ok(())
	}
}
#[async_trait]
impl<S: AnyBlockStorage> StateResolver<S> for StaticStateResolver<S> {
	async fn resolve_state(
		&self,
		_storage: &S,
		_context: &StateResolverContext,
		heads: &BTreeSet<Cid>,
	) -> Result<Option<(Cid, BTreeSet<Cid>)>, anyhow::Error> {
		for (snapshot_state, snapshot_heads) in &self.snapshots {
			if snapshot_heads == heads {
				return Ok(Some((*snapshot_state, heads.clone())));
			}
		}
		Ok(None)
	}

	fn provide_roots(
		&mut self,
		_storage: &S,
		_context: &StateResolverContext,
	) -> Option<BoxStream<'static, Result<(Option<Cid>, BTreeSet<Cid>), anyhow::Error>>> {
		Some(
			stream::iter(self.snapshots.clone())
				.map(|(state, heads)| (Some(state), heads))
				.map(Ok)
				.boxed(),
		)
	}

	async fn push_state(&mut self, _storage: &S, state: Cid, heads: BTreeSet<Cid>) -> Result<(), anyhow::Error> {
		if self.snapshots.iter().find(|item| item.1 == heads).is_none() {
			self.push(state, heads);
		}
		Ok(())
	}

	fn clear(&mut self) {
		self.snapshots.clear();
	}
}

/// Extract head clock.
///
/// # Note
/// We only use the clock from the first head as conflicting heads are expected to have the same clock.
async fn heads_clock(storage: &impl AnyBlockStorage, heads: &BTreeSet<Cid>) -> Result<u64, StorageError> {
	if let Some(head) = heads.first() {
		let entry: SignedEntry = storage.get_deserialized(head).await?;
		return Ok(entry.entry.clock.time);
	} else {
		Err(StorageError::InvalidArgument(anyhow!("heads empty")))
	}
}
