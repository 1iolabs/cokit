use co_primitives::{Link, Node, NodeContainer};
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use futures::{Future, FutureExt, Stream};
use libipld::Cid;
use serde::de::DeserializeOwned;
use std::{
	collections::VecDeque,
	pin::Pin,
	task::{Context, Poll},
};

pub struct NodeStream<S, T> {
	storage: S,
	stack: VecDeque<Cid>,
	entries: VecDeque<T>,
	get: Option<Pin<Box<dyn Future<Output = Result<Node<T>, StorageError>> + Send>>>,
}
impl<S, T> NodeStream<S, T>
where
	S: BlockStorage + Sync + Send + Clone + Unpin + 'static,
	T: DeserializeOwned + Send + Sync + Unpin + 'static,
{
	pub fn new(storage: S, cid: Option<Cid>) -> Self {
		let mut stack = VecDeque::new();
		if let Some(cid) = cid {
			stack.push_front(cid);
		}
		Self { storage, stack, entries: Default::default(), get: None }
	}

	pub fn from_link(storage: S, link: Option<Link<T>>) -> Self {
		Self::new(storage, link.map(|inner| inner.into()))
	}

	pub fn from_node_container<N: NodeContainer<T>>(storage: S, container: &N) -> Self {
		Self::from_link(storage, container.node_container_link())
	}
}
impl<S, T> Stream for NodeStream<S, T>
where
	S: BlockStorage + Send + Sync + Clone + Unpin + 'static,
	T: DeserializeOwned + Send + Sync + Unpin + 'static,
{
	type Item = Result<T, StorageError>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		// waiting?
		if let Some(mut get) = Pin::new(&mut self).get.take() {
			match get.poll_unpin(cx) {
				Poll::Ready(Ok(node)) => match node {
					Node::Node(links) => {
						self.stack.extend(links.into_iter().map(|link| -> Cid { link.into() }));
					},
					Node::Leaf(entries) => self.entries = entries.into(),
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
					return Poll::Pending
				},
			}
		}

		// stack=true entries=false
		if self.entries.is_empty() && !self.stack.is_empty() {
			if let Some(next_cid) = self.stack.pop_front() {
				let storage = self.storage.clone();
				self.get = Some(Box::pin(async move { storage.get_deserialized::<Node<T>>(&next_cid).await }));
				return Poll::Pending;
			}
		}

		// read entry
		Poll::Ready(self.entries.pop_front().map(|entry| Ok(entry)))
	}
}
