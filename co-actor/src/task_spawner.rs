use futures::Future;
use std::sync::Arc;
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
		self.inner
			.spawn(task.instrument(tracing::trace_span!("task", application = self.idenitfier.as_str())))
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
		#[cfg(tokio_unstable)]
		{
			tokio::task::Builder::new()
				.name(name)
				.spawn(self.inner.track_future(task.instrument(tracing::trace_span!(
					"task",
					task_name = name,
					application = self.idenitfier.as_str()
				))))
				.expect("tokio runtime")
		}
		#[cfg(not(tokio_unstable))]
		{
			self.inner.spawn(task.instrument(tracing::trace_span!(
				"task",
				task_name = name,
				application = self.idenitfier.as_str()
			)))
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
