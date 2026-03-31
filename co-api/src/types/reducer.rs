// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::CoreBlockStorage;
use cid::Cid;
use co_primitives::{Link, OptionLink, ReducerAction};

/// COre execution context.
pub trait Context {
	/// Storage instance.
	fn storage(&self) -> &CoreBlockStorage;

	/// Get runtime payload.
	fn payload(&self) -> Vec<u8>;

	/// Get action to apply to the state.
	fn event(&self) -> Cid;

	/// Get current COre state.
	/// Returns [`None`] if no prior state.
	fn state(&self) -> Option<Cid>;

	/// Set next COre state.
	fn set_state(&mut self, cid: Cid);

	/// Write diagnostic block.
	fn write_diagnostic(&mut self, cid: Cid);
}

#[allow(async_fn_in_trait)]
pub trait Reducer<A>
where
	Self: Sized,
	A: Clone,
{
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<A>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error>;
}
