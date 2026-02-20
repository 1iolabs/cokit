use futures::{future::LocalBoxFuture, FutureExt};
use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

/// Spwan a local (not Send) future.
pub trait LocalTaskSpawner {
	fn spawn_local<F>(&self, fut: F) -> LocalJoinHandle<F::Output>
	where
		F: Future + 'static,
		F::Output: Send + 'static;
}
#[derive(Debug, thiserror::Error)]
pub enum LocalJoinError {
	#[error("Task has cancelled")]
	Cancelled,
}
pub struct LocalJoinHandle<O> {
	join: LocalBoxFuture<'static, Result<O, LocalJoinError>>,
}
impl<O> LocalJoinHandle<O> {
	pub fn new(join: impl Future<Output = Result<O, LocalJoinError>> + 'static) -> Self {
		Self { join: join.boxed_local() }
	}
}
impl<O> Future for LocalJoinHandle<O> {
	type Output = Result<O, LocalJoinError>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.join.as_mut().poll(cx)
	}
}
