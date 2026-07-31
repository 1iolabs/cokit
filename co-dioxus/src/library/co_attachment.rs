// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::CoError;
use co_sdk::CoReducerState;
use dioxus::{
	dioxus_core::with_owner,
	signals::{Owner, SyncSignal, SyncStorage, WritableExt},
};
use std::fmt::{Debug, Formatter};

/// Render state of a single CO attachment.
///
/// The signals belong to an owner held by the attachment instead of the component scope, so every
/// piece of background work that kept a clone can still publish into its own state after the
/// component switched to another CO or unmounted.
#[derive(Clone)]
pub struct CoAttachment {
	/// Keeps the signal storage alive while any clone of this attachment exists.
	_owner: Owner<SyncStorage>,
	pub(crate) reducer_state: SyncSignal<Option<Result<CoReducerState, CoError>>>,
	pub(crate) last_error: SyncSignal<Result<(), CoError>>,
}
impl CoAttachment {
	/// Allocate a fresh reducer state and error signal pair under a new owner.
	pub(crate) fn new() -> Self {
		let owner = Owner::<SyncStorage>::default();
		let (reducer_state, last_error) =
			with_owner(owner.clone(), || (SyncSignal::new_maybe_sync(None), SyncSignal::new_maybe_sync(Ok(()))));
		Self { _owner: owner, reducer_state, last_error }
	}

	/// Publish the reducer state of this attachment.
	pub(crate) fn set_reducer_state(&self, reducer_state: Result<CoReducerState, CoError>) {
		let mut signal = self.reducer_state;
		signal.set(Some(reducer_state));
	}

	/// Publish an error of this attachment.
	pub(crate) fn set_last_error(&self, error: CoError) {
		let mut signal = self.last_error;
		signal.set(Err(error));
	}
}
impl Debug for CoAttachment {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CoAttachment")
			.field("reducer_state", &self.reducer_state)
			.field("last_error", &self.last_error)
			.finish_non_exhaustive()
	}
}
