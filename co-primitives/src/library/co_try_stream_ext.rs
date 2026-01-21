use futures::{pin_mut, Stream, TryStreamExt};
use std::{marker::PhantomData, task::Poll};

#[async_trait::async_trait]
pub trait CoTryStreamExt: Stream<Item = Result<Self::Ok, Self::Error>> {
	type Ok;
	type Error;

	async fn try_first(self) -> Result<Option<Self::Ok>, Self::Error>
	where
		Self: Sized,
	{
		Ok(try_first(self).await?)
	}

	/// Ignore all elements by only forwarding errors.
	fn try_ignore_elements<T>(self) -> TryIgnoreElements<Self, T>
	where
		Self: Sized,
	{
		TryIgnoreElements { inner: self, _out: PhantomData }
	}
}
impl<S, T, E> CoTryStreamExt for S
where
	S: ?Sized + Stream<Item = Result<T, E>>,
{
	type Ok = T;
	type Error = E;
}

async fn try_first<T, E, S>(stream: S) -> Result<Option<T>, E>
where
	S: Stream<Item = Result<T, E>> + Sized,
{
	pin_mut!(stream);
	stream.try_next().await
}

#[pin_project::pin_project]
pub struct TryIgnoreElements<S, O> {
	#[pin]
	inner: S,
	_out: PhantomData<O>,
}
impl<S, T, E, O> Stream for TryIgnoreElements<S, O>
where
	S: Stream<Item = Result<T, E>>,
{
	type Item = Result<O, E>;

	fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
		let mut this = self.project();
		match this.inner.as_mut().poll_next(cx) {
			// ignore elements
			Poll::Ready(Some(Ok(_))) => Poll::Pending,
			// forward error
			Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
			// forward comlete
			Poll::Ready(None) => Poll::Ready(None),
			// forward pending
			Poll::Pending => Poll::Pending,
		}
	}
}
