// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::CoreBlockStorage;
use cid::Cid;
use std::collections::BTreeSet;

#[allow(async_fn_in_trait)]
pub trait Guard {
	/// Verify `next_head` is allowed to integrate into `state`@`heads`.
	/// Return `true` if is allowed to integrate, `false` if is not allowed to integrate.
	/// Errors will be treated as not allowed to integrate (`false`) but provide additional context.
	async fn verify(
		storage: &CoreBlockStorage,
		guard: String,
		state: Cid,
		heads: BTreeSet<Cid>,
		next_head: Cid,
	) -> Result<bool, anyhow::Error>;
}
