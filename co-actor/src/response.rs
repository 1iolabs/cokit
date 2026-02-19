// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use super::ActorError;
use crate::TaskSpawner;
use futures::{FutureExt, Sink, Stream};
use std::{
	any::type_name,
	borrow::Borrow,
	fmt::Debug,
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};
use tokio::sync::{mpsc, oneshot};

/// Response.
///
/// # Notes
/// - When the response is dropped inside the actor and has not been used we receive a canceled on the caller side.
#[must_use]
pub struct Response<T> {
	tx: oneshot::Sender<T>,
}
impl<T> Response<T> {
	/// Send Response to caller.
	///
	/// # Notes
	/// - Ignores when the caller is not waiting for the response. When you want to handle this use [`Response::send`].
	pub fn respond(self, value: T) {
		self.send(value).ok();
	}

	/// Executes closure and respond with the result
	pub async fn respond_execute<Fut, F>(self, value: F)
	where
		Fut: Future<Output = T> + Send,
		F: FnOnce() -> Fut + Send,
	{
		self.respond(value().await)
	}

	/// Try to send response to caller.
	///
	/// # Errors
	/// - Fails with [`ActorError::Canceled`] when the caller is not waiting for the result.
	pub fn send(self, value: T) -> Result<(), ActorError> {
		self.tx.send(value).map_err(|_| ActorError::Canceled)
	}

	/// Executes closure and respond with the result
	pub async fn execute<Fut, F>(self, value: F) -> Result<(), ActorError>
	where
		Fut: Future<Output = T> + Send,
		F: FnOnce() -> Fut + Send,
	{
		self.send(value().await)
	}

	/// Spawns a new task and executes given closure in it
	#[inline]
	#[track_caller]
	pub fn spawn<Fut, F>(self, value: F)
	where
		Fut: Future<Output = T> + Send + 'static,
		F: FnOnce() -> Fut + Send + 'static,
		T: Send + 'static,
	{
		Self::spawn_with(self, TaskSpawner::default(), value);
	}

	/// Spawns a new task using the given spawner and executes given closure in it
	#[inline]
	#[track_caller]
	pub fn spawn_with<Fut, F>(self, spawner: impl Borrow<TaskSpawner>, value: F)
	where
		Fut: Future<Output = T> + Send + 'static,
		F: FnOnce() -> Fut + Send + 'static,
		T: Send + 'static,
	{
		spawner.borrow().spawn(async move { self.send(value().await).ok() });
	}
}
impl<T> Debug for Response<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Response")
			.field("response_type", &type_name::<T>())
			.field("tx_closed", &self.tx.is_closed())
			.finish()
	}
}

pub struct ResponseReceiver<T> {
	rx: oneshot::Receiver<T>,
}
impl<T> ResponseReceiver<T> {
	pub fn new() -> (Response<T>, ResponseReceiver<T>) {
		let (tx, rx) = oneshot::channel();
		(Response { tx }, ResponseReceiver { rx })
	}
}
impl<T> Future for ResponseReceiver<T> {
	type Output = Result<T, ActorError>;

	fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.rx.poll_unpin(cx).map_err(|_e| ActorError::Canceled)
	}
}

/// A streaming response.
#[must_use]
pub struct ResponseStream<T> {
	tx: mpsc::UnboundedSender<T>,
}
impl<T> ResponseStream<T> {
	pub fn send(&mut self, value: T) -> Result<(), ActorError> {
		self.tx.send(value).map_err(|_| ActorError::Canceled)
	}

	/// Test if the stream has been closed by the caller.
	pub fn is_closed(&self) -> bool {
		self.tx.is_closed()
	}

	pub fn complete(self) -> Result<(), ActorError> {
		// will be closed on drop
		Ok(())
	}
}
impl<T> Sink<T> for ResponseStream<T> {
	type Error = T;

	fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
		self.get_mut().tx.send(item).map_err(|err| err.0)
	}

	fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}
}
impl<T> Debug for ResponseStream<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ResponseStream")
			.field("response_type", &type_name::<T>())
			.field("tx_closed", &self.tx.is_closed())
			.finish()
	}
}

pub struct ResponseStreamReceiver<T> {
	rx: mpsc::UnboundedReceiver<T>,
}
impl<T> ResponseStreamReceiver<T> {
	pub fn new() -> (ResponseStream<T>, ResponseStreamReceiver<T>) {
		let (tx, rx) = mpsc::unbounded_channel();
		(ResponseStream { tx }, ResponseStreamReceiver { rx })
	}
}
impl<T> Stream for ResponseStreamReceiver<T> {
	type Item = T;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.rx.poll_recv(cx)
	}
}

/// A streaming response with backpressure (bounded).
#[must_use]
pub struct ResponseBackPressureStream<T> {
	tx: mpsc::Sender<Result<T, ActorError>>,
}
impl<T> ResponseBackPressureStream<T> {
	pub async fn send(&mut self, value: T) -> Result<(), ActorError> {
		self.tx.send(Ok(value)).await.map_err(|_| ActorError::Canceled)
	}

	/// Test if the stream has been closed by the caller.
	pub fn is_closed(&self) -> bool {
		self.tx.is_closed()
	}

	pub fn complete(self) -> Result<(), ActorError> {
		// will be closed on drop
		Ok(())
	}
}
impl<T> Debug for ResponseBackPressureStream<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ResponseBackPressureStream")
			.field("response_type", &type_name::<T>())
			.field("tx_closed", &self.tx.is_closed())
			.finish()
	}
}

pub struct ResponseBackPressureStreamReceiver<T> {
	rx: mpsc::Receiver<Result<T, ActorError>>,
}
impl<T> ResponseBackPressureStreamReceiver<T> {
	pub fn new(buffer: usize) -> (ResponseBackPressureStream<T>, ResponseBackPressureStreamReceiver<T>) {
		let (tx, rx) = mpsc::channel(buffer);
		(ResponseBackPressureStream { tx }, ResponseBackPressureStreamReceiver { rx })
	}
}
impl<T: Debug> Stream for ResponseBackPressureStreamReceiver<T> {
	type Item = Result<T, ActorError>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.rx.poll_recv(cx)
	}
}

#[derive(Debug)]
pub struct ResponseStreams<T> {
	streams: Vec<ResponseStream<T>>,
}
impl<T> Default for ResponseStreams<T> {
	fn default() -> Self {
		Self { streams: Default::default() }
	}
}
impl<T> ResponseStreams<T>
where
	T: Clone,
{
	pub fn push(&mut self, stream: ResponseStream<T>) {
		self.streams.push(stream);
	}

	pub fn send(&mut self, value: T) {
		self.streams
			.retain_mut(|stream| !matches!(stream.send(value.clone()), Err(ActorError::Canceled)));
	}

	/// Test if the streams has been closed by the caller.
	pub fn is_closed(&self) -> bool {
		!self.streams.iter().any(|s| !s.is_closed())
	}
}
