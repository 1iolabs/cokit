// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::task_handle::TaskHandle;
use std::future::Future;

pub type LocalTaskHandle<T> = TaskHandle<T>;

/// Spawn a local (not Send) future.
pub trait LocalTaskSpawner {
	fn spawn_local<F>(&self, fut: F) -> LocalTaskHandle<F::Output>
	where
		F: Future + 'static,
		F::Output: Send + 'static;
}
