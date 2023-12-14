use super::entry::EntryBlock;
use libipld::Cid;
use std::collections::BTreeSet;

/// Find heads.
pub fn find_heads(entries: impl Iterator<Item = EntryBlock>) -> Vec<EntryBlock> {
	let mut all_next_cids: BTreeSet<Cid> = Default::default();
	let mut all_entries: BTreeSet<EntryBlock> = Default::default();
	for entry in entries {
		for next in entry.entry().next.iter() {
			all_next_cids.insert(next.clone());
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
