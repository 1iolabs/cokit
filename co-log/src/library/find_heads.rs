// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::entry::EntryBlock;
use cid::Cid;
use std::collections::BTreeSet;

/// Find heads. Which are the "loose ends" of all items in `entries`.
pub fn find_heads<'a>(entries: impl Iterator<Item = &'a EntryBlock>) -> Vec<&'a EntryBlock> {
	let mut all_next_cids: BTreeSet<Cid> = Default::default();
	let mut all_entries: BTreeSet<&'a EntryBlock> = Default::default();
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
