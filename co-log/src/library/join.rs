use super::{
	find_heads::find_heads,
	get_entry_block::{get_entry_block, get_entry_blocks},
	verify_entry::verify_entry,
};
use crate::{EntryBlock, Log, LogError};
use co_storage::BlockStorage;
use libipld::Cid;
use std::{collections::BTreeSet, marker::PhantomData};

pub struct JoinEntry<S> {
	_storage_type: PhantomData<S>,

	/// Added entries
	entries_to_add: BTreeSet<Cid>,

	/// Resulting heads.
	heads: BTreeSet<Cid>,
}
impl<'a, S> JoinEntry<S>
where
	S: BlockStorage + Send + Sync + 'a,
{
	pub fn new(heads: BTreeSet<Cid>) -> Self {
		Self { _storage_type: Default::default(), entries_to_add: Default::default(), heads }
	}

	/// Join entry.
	///
	/// Returns true if the other heads are joined or if this call caused to load entries from storage.
	/// We can not compute if there has been changes without loading the whole log.
	///
	/// TODO: validate entry signatures?
	pub async fn join_entry(&mut self, log: &'a Log<S>, entry: EntryBlock<S::StoreParams>) -> Result<bool, LogError> {
		// contains?
		if log.contains(entry.cid()) && self.entries_to_add.contains(entry.cid()) {
			return Ok(!self.entries_to_add.is_empty())
		}

		// verify
		verify_entry(log, &entry)?;

		// calculate
		let mut entries_to_get: BTreeSet<Cid> = Default::default();
		let mut connected_heads: BTreeSet<Cid> = Default::default();
		self.entries_to_add.insert(entry.cid().clone());
		entries_to_get.extend(entry.entry().next.iter());
		entries_to_get.extend(entry.entry().refs.iter());
		while !entries_to_get.is_empty() {
			// TODO: prefetch
			// self.entry_store.fetch(entries_to_get.iter())
			while let Some(cid) = entries_to_get.pop_first() {
				let e = get_entry_block(log.storage(), &cid).await?;
				verify_entry(log, &e)?;
				self.entries_to_add.insert(e.cid().clone());

				for next in e.entry().next.iter().chain(e.entry().refs.iter()) {
					if !log.contains(next) && !self.entries_to_add.contains(next) {
						entries_to_get.insert(next.clone());
					} else if self.heads.contains(next) {
						connected_heads.insert(next.clone());
					}
				}
			}
		}

		// heads
		let possible_heads = get_entry_blocks(
			log.storage(),
			self.heads
				.iter()
				// skip connected entries
				.filter(|cid| !connected_heads.contains(cid))
				// also resolve entry
				.chain([entry.cid().clone()].iter()),
		)
		.await?;
		self.heads = find_heads(possible_heads.iter()).iter().map(|e| e.cid().clone()).collect();

		// result
		Ok(true)
	}

	pub fn into_inner(self) -> (BTreeSet<Cid>, BTreeSet<Cid>) {
		(self.heads, self.entries_to_add)
	}
}
