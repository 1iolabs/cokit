// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use futures::Future;
use std::{panic::Location, sync::Arc};
use tokio::task::JoinHandle;
use tokio_util::task::TaskTracker;
use tracing::Instrument;

#[derive(Debug, Clone)]
pub struct TaskSpawner {
	pub(crate) idenitfier: Arc<String>,
	pub(crate) inner: TaskTracker,
}
impl TaskSpawner {
	pub fn new(idenitfier: String, inner: TaskTracker) -> Self {
		Self { idenitfier: Arc::new(idenitfier), inner }
	}

	/// Spawn task.
	#[inline]
	#[track_caller]
	pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
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
		self.inner.spawn(task.instrument(span))
	}

	/// Spawn task.
	#[inline]
	#[track_caller]
	#[allow(unexpected_cfgs)]
	pub fn spawn_named<F>(&self, name: &str, task: F) -> JoinHandle<F::Output>
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
		#[cfg(tokio_unstable)]
		{
			tokio::task::Builder::new()
				.name(name)
				.spawn(self.inner.track_future(task.instrument(span)))
				.expect("tokio runtime")
		}
		#[cfg(not(tokio_unstable))]
		{
			self.inner.spawn(task.instrument(span))
		}
	}

	pub fn tracker(&self) -> TaskTracker {
		self.inner.clone()
	}
}
impl Default for TaskSpawner {
	fn default() -> Self {
		Self { idenitfier: Arc::new("default".to_string()), inner: Default::default() }
	}
}
