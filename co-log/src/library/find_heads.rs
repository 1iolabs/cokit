use super::entry::EntryBlock;
use libipld::{store::StoreParams, Cid};
use std::collections::BTreeSet;

/// Find heads. Which are the "loose ends" of all items in `entries`.
pub fn find_heads<'a, P: StoreParams>(entries: impl Iterator<Item = &'a EntryBlock<P>>) -> Vec<&'a EntryBlock<P>> {
	let mut all_next_cids: BTreeSet<Cid> = Default::default();
	let mut all_entries: BTreeSet<&'a EntryBlock<P>> = Default::default();
	for entry in entries {
		for next in entry.entry().next.iter() {
			all_next_cids.insert(*next);
		}
		all_entries.insert(entry);
	}

	let mut result = Vec::new();
	for entry in all_entries.into_iter() {
		if !all_next_cids.contains(entry.cid()) {
			result.push(entry)
		}
	}

	result
}
