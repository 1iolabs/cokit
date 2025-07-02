use cid::Cid;
use co_primitives::BlockStorage;
use std::collections::BTreeSet;

#[allow(async_fn_in_trait)]
pub trait Guard<S>
where
	S: BlockStorage + Clone + 'static,
{
	/// Verify `next_head` is allowed to integrate into `state`@`heads`.
	/// Return `true` if is allowed to integrate, `false` if is not allowed to integrate.
	/// Errors will be treated as not allowed to integrate (`false`) but provide additional context.
	async fn verify(
		storage: &S,
		guard: String,
		state: Cid,
		heads: BTreeSet<Cid>,
		next_head: Cid,
	) -> Result<bool, anyhow::Error>;
}
