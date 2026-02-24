// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_log::EntryBlock;
use co_storage::BlockStorage;
use std::collections::BTreeSet;

/// Extract all `next` heads from given heads.
pub async fn extract_next_heads<S>(
	storage: &S,
	heads: impl IntoIterator<Item = &Cid>,
	include_refs: bool,
) -> Result<BTreeSet<Cid>, anyhow::Error>
where
	S: BlockStorage,
{
	let mut next = BTreeSet::new();
	for head in heads.into_iter() {
		let head_block = storage.get(head).await?;
		let entry = EntryBlock::from_block(head_block)?;
		next.extend(entry.entry().next.iter().cloned());
		if include_refs {
			next.extend(entry.entry().refs.iter().cloned());
		}
	}
	Ok(next)
}
