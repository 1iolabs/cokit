use futures::{stream::FusedStream, Stream, StreamExt};
use std::{
	fmt::Debug,
	pin::Pin,
	task::{Context, Poll},
};

#[derive(Debug)]
pub struct Observable<T>
where
	T: Clone + Send + 'static,
{
	sender: tokio::sync::broadcast::Sender<Message<T>>,
	subscription: Option<tokio_stream::wrappers::BroadcastStream<Message<T>>>,
	complete: bool,
}
impl<T> Observable<T>
where
	T: Clone + Send + 'static,
{
	pub fn new() -> Self {
		let (tx, _) = tokio::sync::broadcast::channel(64);
		Self { complete: false, sender: tx, subscription: None }
	}

	pub fn dispatch(&self, value: T) {
		self.sender.send(Message::Message(value)).ok();
	}

	pub fn shutdown(&self) {
		self.sender.send(Message::Shutdown).ok();
	}
}
impl<T> Default for Observable<T>
where
	T: Clone + Send + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}
impl<T> Stream for Observable<T>
where
	T: Clone + Send + 'static,
{
	type Item = T;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		// subscribe on first poll
		if self.subscription.is_none() {
			self.subscription = Some(self.sender.subscribe().into());
		}

		// forward
		match self.subscription.as_mut().unwrap().poll_next_unpin(cx) {
			Poll::Ready(Some(Ok(Message::Message(v)))) => Poll::Ready(Some(v)),
			Poll::Ready(Some(Ok(Message::Shutdown))) => {
				self.complete = true;
				Poll::Ready(None)
			},
			Poll::Ready(Some(Err(_))) => Poll::Ready(None),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}
impl<T> Clone for Observable<T>
where
	T: Clone + Send + 'static,
{
	fn clone(&self) -> Self {
		Self { sender: self.sender.clone(), subscription: None, complete: self.complete.clone() }
	}
}
impl<T> FusedStream for Observable<T>
where
	T: Clone + Send + 'static,
{
	fn is_terminated(&self) -> bool {
		self.complete
	}
}

#[derive(Debug, Clone)]
enum Message<T> {
	Message(T),
	Shutdown,
}
