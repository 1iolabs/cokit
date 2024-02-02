use super::conflict::last_write_wins;
use crate::{
	library::get_entry_block::{get_entry_block, get_entry_blocks},
	EntryBlock, LogError,
};
use anyhow::Context;
use co_storage::{BlockStorage, Storage};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use libipld::Cid;
use std::collections::{BTreeSet, HashSet};

pub fn create_stream<'a, S>(
	storage: &'a S,
	heads: BTreeSet<Cid>,
) -> impl Stream<Item = Result<EntryBlock<S::StoreParams>, LogError>> + 'a
where
	S: BlockStorage + Sync + Send + 'a,
{
	async_stream::try_stream! {
		let mut traversed: HashSet<Cid> = Default::default();
		let mut stack = get_entry_blocks(storage, heads.iter()).await?;

		while !stack.is_empty() {
			// sort stack
			stack.sort_by(last_write_wins);

			// stack
			if let Some(entry) = stack.pop() {
				if !traversed.contains(entry.cid()) {
					// flag as traversed
					traversed.insert(entry.cid().clone());

					// TODO: (pre) fetch refs
					// self.storage.fetch(entry.entry().next.iter());
					// self.storage.fetch(entry.entry().refs.iter());

					// read next and add to stack
					let mut nexts: Vec<EntryBlock<S::StoreParams>> = stream::iter(entry.entry().next.iter())
						.then(|cid| async move { get_entry_block(storage, &cid).await })
						.try_collect()
						.await?;
					stack.append(&mut nexts);

					// result
					yield entry;
				}
			}
		}
	}
}

pub struct LogIterator<S>
where
	S: Storage,
{
	storage: S,
	stack: Vec<EntryBlock<S::StoreParams>>,
	error: Option<anyhow::Error>,
	traversed: HashSet<Cid>,
}
impl<S> LogIterator<S>
where
	S: Storage,
{
	pub fn new(storage: S, stack: Vec<EntryBlock<S::StoreParams>>) -> Self {
		LogIterator { storage, stack, error: None, traversed: Default::default() }
	}

	fn sort(&mut self) {
		self.stack.sort_by(last_write_wins);
	}
}
impl<S> Iterator for LogIterator<S>
where
	S: Storage,
{
	type Item = Result<EntryBlock<S::StoreParams>, anyhow::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		// error?
		if let Some(e) = self.error.take() {
			// clear stack because we are done after an error
			self.stack.clear();

			// return error
			return Some(Err(e))
		}

		// sort stack
		//  TODO: do we need to consider causality or is the clock enought?
		self.sort();

		// stack
		if let Some(entry) = self.stack.pop() {
			if !self.traversed.contains(entry.cid()) {
				// flag as traversed
				self.traversed.insert(entry.cid().clone());

				// TODO: (pre) fetch refs
				// self.storage.fetch(entry.entry().next.iter());
				// self.storage.fetch(entry.entry().refs.iter());

				// read next and add to stack
				let nexts: Result<Vec<EntryBlock<S::StoreParams>>, anyhow::Error> = entry
					.entry()
					.next
					.iter()
					.map(|cid| -> Result<EntryBlock<S::StoreParams>, anyhow::Error> {
						match self.storage.get(cid).context("Get entry from storage") {
							Ok(block) => EntryBlock::from_block(block).context("Validate block"),
							Err(e) => Err(e),
						}
					})
					.collect();
				match nexts {
					Ok(mut i) => self.stack.append(&mut i),
					Err(e) => self.error = Some(e),
				}

				// result
				return Some(Ok(entry))
			}
		}

		None
	}
}
