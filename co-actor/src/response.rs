use super::ActorError;
use crate::TaskSpawner;
use futures::{FutureExt, Stream};
use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct Response<T> {
	tx: oneshot::Sender<T>,
}
impl<T> Response<T> {
	pub fn respond(self, value: T) -> Result<(), ActorError> {
		self.tx.send(value).map_err(|_| ActorError::Canceled)
	}

	/// Alias to respond.
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
	pub fn spawn<Fut, F>(self, value: F)
	where
		Fut: Future<Output = T> + Send + 'static,
		F: FnOnce() -> Fut + Send + 'static,
		T: Send + 'static,
	{
		Self::spawn_with(self, Default::default(), value);
	}

	/// Spawns a new task using the given spawner and executes given closure in it
	pub fn spawn_with<Fut, F>(self, spawner: TaskSpawner, value: F)
	where
		Fut: Future<Output = T> + Send + 'static,
		F: FnOnce() -> Fut + Send + 'static,
		T: Send + 'static,
	{
		spawner.spawn(async move { self.send(value().await).ok() });
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
		self.rx.poll_unpin(cx).map_err(|e| ActorError::InvalidState(e.into()))
	}
}

#[derive(Debug)]
/// A streaming response.
pub struct ResponseStream<T> {
	tx: mpsc::UnboundedSender<Result<T, ActorError>>,
}
impl<T> ResponseStream<T> {
	pub fn send(&mut self, value: T) -> Result<(), ActorError> {
		self.tx.send(Ok(value)).map_err(|_| ActorError::Canceled)
	}

	pub fn complete(self) -> Result<(), ActorError> {
		// will be closed on drop
		Ok(())
	}
}

pub struct ResponseStreamReceiver<T> {
	rx: mpsc::UnboundedReceiver<Result<T, ActorError>>,
}
impl<T> ResponseStreamReceiver<T> {
	pub fn new() -> (ResponseStream<T>, ResponseStreamReceiver<T>) {
		let (tx, rx) = mpsc::unbounded_channel();
		(ResponseStream { tx }, ResponseStreamReceiver { rx })
	}
}
impl<T> Stream for ResponseStreamReceiver<T> {
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
		self.streams.retain_mut(|stream| match stream.send(value.clone()) {
			Err(ActorError::Canceled) => false,
			_ => true,
		});
	}
}
