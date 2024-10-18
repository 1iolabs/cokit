use futures::Future;
use tokio::task::JoinHandle;
use tokio_util::task::TaskTracker;
use tracing::Instrument;

#[derive(Debug, Clone)]
pub struct TaskSpawner {
	pub(crate) idenitfier: String,
	pub(crate) inner: TaskTracker,
}
impl TaskSpawner {
	pub fn new(idenitfier: String, inner: TaskTracker) -> Self {
		Self { idenitfier, inner }
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
			.spawn(task.instrument(tracing::trace_span!("task", application = self.idenitfier)))
	}

	pub fn tracker(&self) -> TaskTracker {
		self.inner.clone()
	}
}
impl Default for TaskSpawner {
	fn default() -> Self {
		Self { idenitfier: "default".to_string(), inner: Default::default() }
	}
}
