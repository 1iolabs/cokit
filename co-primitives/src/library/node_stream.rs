// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
pub struct NodeStream<S, T, N = Node<T>>
where
	N: NodeReader<T>,
{
	storage: S,
	stack: VecDeque<Cid>,
	entries: VecDeque<T>,
	#[pin]
	get: Option<Pin<Box<dyn Future<Output = Result<N, StorageError>> + Send>>>,
	filter: N::Filter,
	reverse: bool,
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
		Self { storage, stack, entries: Default::default(), get: None, filter: Default::default(), reverse: false }
	}

	pub fn from_link(storage: S, link: OptionLink<N>) -> Self {
		Self::new(storage, *link.cid())
	}

	pub fn from_node(storage: S, node: N, filter: Option<N::Filter>) -> Self {
		let filter = filter.unwrap_or_default();
		let (stack, entries) = match node.read(&filter) {
			Either::Left(stack) => (stack.into_iter().collect(), Default::default()),
			Either::Right(entries) => (Default::default(), entries.into_iter().collect()),
		};
		Self { storage, stack, entries, get: None, filter, reverse: false }
	}

	/// Iterate with filter.
	pub fn with_filter(mut self, filter: N::Filter) -> Self {
		self.filter = filter;
		self
	}

	/// Iterate in reverse order.
	pub fn with_reverse(mut self) -> Self {
		self.reverse = true;
		self
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
				if let Some(next_cid) = if self.reverse { self.stack.pop_back() } else { self.stack.pop_front() } {
					let storage = self.storage.clone();
					self.get = Some(Box::pin(async move { storage.get_deserialized::<N>(&next_cid).await }));
				}
			}

			// waiting?
			if let Some(mut get) = Pin::new(&mut self).get.take() {
				match get.poll_unpin(cx) {
					Poll::Ready(Ok(node)) => match node.read(&self.filter) {
						Either::Left(links) => {
							self.stack.extend(links);
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
		Poll::Ready(
			if self.reverse { self.entries.pop_back() } else { self.entries.pop_front() }.map(|entry| Ok(entry)),
		)
	}
}

#[cfg(test)]
mod tests {
	use crate::{library::test::TestStorage, BlockStorage, DefaultNodeSerializer, NodeBuilder, NodeStream};
	use futures::TryStreamExt;

	#[tokio::test]
	async fn test_stream() {
		let storage = TestStorage::default();

		// build
		let mut builder = NodeBuilder::new(storage.max_block_size(), 2, DefaultNodeSerializer::new());
		for i in 0..10 {
			builder.push(i).unwrap();
		}
		let (root, blocks) = builder.into_blocks().unwrap();
		for block in blocks {
			storage.set(block).await.unwrap();
		}

		// stream
		let list = NodeStream::from_link(storage.clone(), root)
			.try_collect::<Vec<i32>>()
			.await
			.unwrap();
		assert_eq!(list[..], [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
	}

	#[tokio::test]
	async fn test_stream_reverse() {
		let storage = TestStorage::default();

		// build
		let mut builder = NodeBuilder::new(storage.max_block_size(), 2, DefaultNodeSerializer::new());
		for i in 0..10 {
			builder.push(i).unwrap();
		}
		let (root, blocks) = builder.into_blocks().unwrap();
		for block in blocks {
			storage.set(block).await.unwrap();
		}

		// stream
		let list = NodeStream::from_link(storage.clone(), root)
			.with_reverse()
			.try_collect::<Vec<i32>>()
			.await
			.unwrap();
		assert_eq!(list[..], [9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
	}
}
