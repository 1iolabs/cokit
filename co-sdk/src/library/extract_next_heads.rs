use cid::Cid;
use co_core_co::Co;
use co_log::EntryBlock;
use co_primitives::AnyBlockStorage;
use co_storage::{BlockStorage, BlockStorageExt};
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

/// Extract `next` state from given state.
pub async fn extract_next_state(
	storage: &impl AnyBlockStorage,
	state: &Option<Cid>,
) -> Result<BTreeSet<Cid>, anyhow::Error> {
	Ok(if let Some(state) = state {
		if let Some(co) = storage.get_deserialized::<Co>(state).await.ok() {
			co.next.cid().to_owned()
		} else {
			None
		}
	} else {
		None
	}
	.into_iter()
	.collect())
}
