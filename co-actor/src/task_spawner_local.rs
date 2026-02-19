// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use futures::{future::LocalBoxFuture, FutureExt};
use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

/// Spwan a local (not Send) future.
pub trait LocalTaskSpawner {
	fn spwan_local<F>(&self, fut: F) -> LocalJoinHandle<F::Output>
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
