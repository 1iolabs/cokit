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

	/// Subscribe to all items send after this call.
	/// This should be only used for synchronisation purposes.
	/// When not used the subscribition start with first strem polling.
	/// Note: This will buffer all stream events until read and may causes the buffer to be full.
	pub fn subscribe(mut self) -> Self {
		if self.subscription.is_none() {
			self.subscription = Some(self.sender.subscribe().into());
		}
		self
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

#[cfg(test)]
mod tests {
	use crate::Observable;
	use futures::StreamExt;

	#[derive(Debug, Clone, PartialEq)]
	enum Action {
		A,
		B,
	}

	#[tokio::test]
	async fn smoke() {
		let actions = Observable::new();
		let (result, _) = futures::future::join(actions.clone().collect::<Vec<Action>>(), async move {
			actions.dispatch(Action::A);
			actions.dispatch(Action::B);
			actions.shutdown();
		})
		.await;
		assert_eq!(result, vec![Action::A, Action::B]);
	}

	#[tokio::test]
	async fn test_clone() {
		let actions = Observable::new();
		let (result1, result2, _) = futures::future::join3(
			actions.clone().collect::<Vec<Action>>(),
			actions.clone().collect::<Vec<Action>>(),
			async move {
				actions.dispatch(Action::A);
				actions.dispatch(Action::B);
				actions.shutdown();
			},
		)
		.await;
		assert_eq!(result1, vec![Action::A, Action::B]);
		assert_eq!(result2, vec![Action::A, Action::B]);
	}

	#[tokio::test]
	async fn test_subscribe() {
		let actions = Observable::new();
		let output = actions.clone().subscribe();
		let handle = tokio::spawn(async move { output.collect::<Vec<Action>>().await });
		actions.dispatch(Action::A);
		actions.dispatch(Action::B);
		actions.shutdown();
		assert_eq!(handle.await.unwrap(), vec![Action::A, Action::B]);
	}
}
