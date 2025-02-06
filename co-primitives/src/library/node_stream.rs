use super::node_builder::NodeReader;
use crate::{BlockStorage, BlockStorageExt, Node, OptionLink, StorageError};
use cid::Cid;
use either::Either;
use futures::{Future, FutureExt, Stream};
use pin_project::pin_project;
use serde::de::DeserializeOwned;
use std::{
	collections::VecDeque,
	pin::Pin,
	task::{Context, Poll},
};

/// Stream node items.
#[pin_project]
pub struct NodeStream<S, T, N = Node<T>> {
	storage: S,
	stack: VecDeque<Cid>,
	entries: VecDeque<T>,
	#[pin]
	get: Option<Pin<Box<dyn Future<Output = Result<N, StorageError>> + Send>>>,
}
impl<S, T, N> NodeStream<S, T, N>
where
	S: BlockStorage + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: NodeReader<T> + DeserializeOwned + Send + Sync + 'static,
{
	pub fn new(storage: S, cid: Option<Cid>) -> Self {
		let mut stack = VecDeque::new();
		if let Some(cid) = cid {
			stack.push_front(cid);
		}
		Self { storage, stack, entries: Default::default(), get: None }
	}

	pub fn from_link(storage: S, link: OptionLink<N>) -> Self {
		Self::new(storage, *link.cid())
	}
}
impl<S, T, N> Stream for NodeStream<S, T, N>
where
	S: BlockStorage + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: NodeReader<T> + DeserializeOwned + Send + Sync + 'static,
{
	type Item = Result<T, StorageError>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		loop {
			// get next?
			if self.entries.is_empty() && !self.stack.is_empty() && self.get.is_none() {
				if let Some(next_cid) = self.stack.pop_front() {
					let storage = self.storage.clone();
					self.get = Some(Box::pin(async move { storage.get_deserialized::<N>(&next_cid).await }));
				}
			}

			// waiting?
			if let Some(mut get) = Pin::new(&mut self).get.take() {
				match get.poll_unpin(cx) {
					Poll::Ready(Ok(node)) => match node.read() {
						Either::Left(links) => {
							self.stack.extend(links.into_iter());
							continue;
						},
						Either::Right(entries) => {
							self.entries = entries.into();
						},
					},
					Poll::Ready(Err(e)) => {
						// clear
						self.stack.clear();
						self.entries.clear();

						// fail
						return Poll::Ready(Some(Err(e)));
					},
					Poll::Pending => {
						self.get = Some(get);
						return Poll::Pending;
					},
				}
			}
			break;
		}

		// read entry
		Poll::Ready(self.entries.pop_front().map(|entry| Ok(entry)))
	}
}

/// Stream node items in reverse order.
#[pin_project]
pub struct ReverseNodeStream<S, T, N = Node<T>> {
	storage: S,
	stack: VecDeque<Cid>,
	entries: VecDeque<T>,
	#[pin]
	get: Option<Pin<Box<dyn Future<Output = Result<N, StorageError>> + Send>>>,
}
impl<S, T, N> ReverseNodeStream<S, T, N>
where
	S: BlockStorage + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: NodeReader<T> + DeserializeOwned + Send + Sync + 'static,
{
	pub fn new(storage: S, cid: Option<Cid>) -> Self {
		let mut stack = VecDeque::new();
		if let Some(cid) = cid {
			stack.push_front(cid);
		}
		Self { storage, stack, entries: Default::default(), get: None }
	}

	pub fn from_link(storage: S, link: OptionLink<N>) -> Self {
		Self::new(storage, *link.cid())
	}
}
impl<S, T, N> Stream for ReverseNodeStream<S, T, N>
where
	S: BlockStorage + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: NodeReader<T> + DeserializeOwned + Send + Sync + 'static,
{
	type Item = Result<T, StorageError>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		loop {
			// get next?
			if self.entries.is_empty() && !self.stack.is_empty() && self.get.is_none() {
				if let Some(next_cid) = self.stack.pop_back() {
					let storage = self.storage.clone();
					self.get = Some(Box::pin(async move { storage.get_deserialized::<N>(&next_cid).await }));
				}
			}

			// waiting?
			if let Some(mut get) = Pin::new(&mut self).get.take() {
				match get.poll_unpin(cx) {
					Poll::Ready(Ok(node)) => match node.read() {
						Either::Left(links) => {
							self.stack.extend(links.into_iter());
							continue;
						},
						Either::Right(entries) => {
							self.entries = entries.into();
						},
					},
					Poll::Ready(Err(e)) => {
						// clear
						self.stack.clear();
						self.entries.clear();

						// fail
						return Poll::Ready(Some(Err(e)));
					},
					Poll::Pending => {
						self.get = Some(get);
						return Poll::Pending;
					},
				}
			}
			break;
		}

		// read entry
		Poll::Ready(self.entries.pop_back().map(|entry| Ok(entry)))
	}
}
