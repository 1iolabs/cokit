// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_actor::{LocalTaskHandle, LocalTaskSpawner};
use std::future::Future;
use wasm_bindgen_futures::spawn_local;

#[derive(Debug, Default, Clone, Copy)]
pub struct JsLocalTaskSpawner {}
impl LocalTaskSpawner for JsLocalTaskSpawner {
	fn spawn_local<F>(&self, fut: F) -> LocalTaskHandle<F::Output>
	where
		F: Future + 'static,
		F::Output: Send + 'static,
	{
		let (task, task_handle) = LocalTaskHandle::handle_local(fut);
		spawn_local(task);
		task_handle
	}
}
