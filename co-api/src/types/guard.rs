// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
