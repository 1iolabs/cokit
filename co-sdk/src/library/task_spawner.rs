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
}
