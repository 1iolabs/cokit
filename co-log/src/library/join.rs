use super::{
	find_heads::find_heads,
	get_entry_block::{get_entry_block, get_entry_blocks},
	verify_entry::verify_entry,
};
use crate::{EntryBlock, Log, LogError};
use cid::Cid;
use co_primitives::BlockStorage;
use std::collections::BTreeSet;

pub struct JoinEntry {
	/// Added entries
	entries_to_add: BTreeSet<Cid>,

	/// Resulting heads.
	heads: BTreeSet<Cid>,
}
impl JoinEntry {
	pub fn new(heads: BTreeSet<Cid>) -> Self {
		Self { entries_to_add: Default::default(), heads }
	}

	/// Join entry.
	///
	/// Returns true if the other heads are joined or if this call caused to load entries from storage.
	/// We can not compute if there has been changes without loading the whole log.
	///
	/// TODO: validate entry signatures?
	pub async fn join_entry<S>(&mut self, storage: &S, log: &Log, entry: EntryBlock) -> Result<bool, LogError>
	where
		S: BlockStorage + Send + Sync + 'static,
	{
		// contains?
		if log.contains(entry.cid()) && self.entries_to_add.contains(entry.cid()) {
			return Ok(!self.entries_to_add.is_empty());
		}

		// verify
		verify_entry(log, &entry).await?;

		// calculate
		let mut entries_to_get: BTreeSet<Cid> = Default::default();
		let mut connected_heads: BTreeSet<Cid> = Default::default();
		self.entries_to_add.insert(*entry.cid());
		entries_to_get.extend(entry.entry().next.iter());
		entries_to_get.extend(entry.entry().refs.iter());
		while !entries_to_get.is_empty() {
			// TODO: prefetch
			// self.entry_store.fetch(entries_to_get.iter())
			while let Some(cid) = entries_to_get.pop_first() {
				let e = get_entry_block(storage, &cid).await?;
				verify_entry(log, &e).await?;
				self.entries_to_add.insert(*e.cid());

				for next in e.entry().next.iter().chain(e.entry().refs.iter()) {
					if !log.contains(next) && !self.entries_to_add.contains(next) {
						entries_to_get.insert(*next);
					} else if self.heads.contains(next) {
						connected_heads.insert(*next);
					}
				}
			}
		}

		// heads
		let possible_heads = get_entry_blocks(
			storage,
			self.heads
				.iter()
				// skip connected entries
				.filter(|cid| !connected_heads.contains(cid))
				// also resolve entry
				.chain([*entry.cid()].iter()),
		)
		.await?;
		self.heads = find_heads(possible_heads.iter()).iter().map(|e| *e.cid()).collect();

		// result
		Ok(true)
	}

	pub fn into_inner(self) -> (BTreeSet<Cid>, BTreeSet<Cid>) {
		(self.heads, self.entries_to_add)
	}
}
