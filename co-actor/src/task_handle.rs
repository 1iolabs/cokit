// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use futures::{
	future::{BoxFuture, LocalBoxFuture},
	FutureExt,
};
use std::{
	fmt::Debug,
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
	#[error("Task has cancelled")]
	Cancelled,
}

pub struct TaskHandle<O: Send + 'static> {
	join: tokio::sync::oneshot::Receiver<O>,
}
impl<O: Send + 'static> Debug for TaskHandle<O> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("TaskHandle").finish()
	}
}
impl<O: Send + 'static> TaskHandle<O> {
	pub fn handle<F>(task: F) -> (BoxFuture<'static, ()>, TaskHandle<O>)
	where
		O: Send + 'static,
		F: Future<Output = O> + Send + 'static,
	{
		let (tx, rx) = tokio::sync::oneshot::channel::<O>();
		let task = async move {
			tx.send(task.await).ok();
		};
		let task_handle = TaskHandle { join: rx };
		(task.boxed(), task_handle)
	}

	pub fn handle_local<F>(task: F) -> (LocalBoxFuture<'static, ()>, TaskHandle<O>)
	where
		O: Send + 'static,
		F: Future<Output = O> + 'static,
	{
		let (tx, rx) = tokio::sync::oneshot::channel::<O>();
		let task = async move {
			tx.send(task.await).ok();
		};
		let task_handle = TaskHandle { join: rx };
		(task.boxed_local(), task_handle)
	}
}
impl<O: Send + 'static> Future for TaskHandle<O> {
	type Output = Result<O, TaskError>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match self.join.poll_unpin(cx) {
			Poll::Ready(Ok(result)) => Poll::Ready(Ok(result)),
			Poll::Ready(Err(_)) => Poll::Ready(Err(TaskError::Cancelled)),
			Poll::Pending => Poll::Pending,
		}
	}
}
static_assertions::assert_impl_all!(TaskHandle<()>: Send);
