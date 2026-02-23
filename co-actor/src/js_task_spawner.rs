pub use crate::task_handle::TaskHandle;
use futures::Future;
use std::{panic::Location, sync::Arc};
use tracing::Instrument;

#[derive(Debug, Clone)]
pub struct TaskSpawner {
	pub(crate) idenitfier: Arc<String>,
}
impl TaskSpawner {
	pub fn new(idenitfier: String) -> Self {
		Self { idenitfier: Arc::new(idenitfier) }
	}

	/// Spawn task.
	#[inline]
	#[track_caller]
	pub fn spawn<F>(&self, task: F) -> TaskHandle<F::Output>
	where
		F: Future + Send + 'static,
		F::Output: Send + 'static,
	{
		let caller_file = Location::caller().file();
		let caller_line = Location::caller().line();
		let caller_column = Location::caller().column();
		let span = tracing::trace_span!(
			"task",
			application = self.idenitfier.as_str(),
			caller_file,
			caller_line,
			caller_column,
		);
		let (task, task_handle) = TaskHandle::handle(task);
		wasm_bindgen_futures::spawn_local(task.instrument(span));
		task_handle
	}

	/// Spawn task.
	#[inline]
	#[track_caller]
	#[allow(unexpected_cfgs)]
	pub fn spawn_named<F>(&self, name: &str, task: F) -> TaskHandle<F::Output>
	where
		F: Future + Send + 'static,
		F::Output: Send + 'static,
	{
		let caller_file = Location::caller().file();
		let caller_line = Location::caller().line();
		let caller_column = Location::caller().column();
		let span = tracing::trace_span!(
			"task",
			task_name = name,
			application = self.idenitfier.as_str(),
			caller_file,
			caller_line,
			caller_column,
		);
		let (task, task_handle) = TaskHandle::handle(task);
		wasm_bindgen_futures::spawn_local(task.instrument(span));
		task_handle
	}
}
impl Default for TaskSpawner {
	fn default() -> Self {
		Self { idenitfier: Arc::new("default".to_string()) }
	}
}
