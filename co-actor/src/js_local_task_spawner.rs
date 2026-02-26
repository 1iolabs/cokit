// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{LocalTaskHandle, LocalTaskSpawner};
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
