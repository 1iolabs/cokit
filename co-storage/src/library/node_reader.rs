use crate::{Storage, StorageError};
use cid::Cid;
use co_primitives::{BlockSerializer, MultiCodec, Node};
use serde::de::DeserializeOwned;
use std::collections::VecDeque;

// pub fn node_reader_fn<T, F>(storage: &dyn Storage, cid: &Cid, f: &F) -> anyhow::Result<()>
// where
// 	T: Clone + DeserializeOwned,
// 	F: Fn(T),
// {
// 	// get block
// 	let block = storage.get(cid)?;
// 	if block.cid().codec() != Into::<u64>::into(DagCborCodec) {
// 		return Err(StorageError::InvalidArgument)?
// 	}

// 	// get node
// 	let node: Node<T> = from_cbor(block.data()).map_err(|_| StorageError::InvalidArgument)?;

// 	// read
// 	match node {
// 		Node::Node(links) =>
// 			for link in links {
// 				node_reader_fn(storage, link.as_ref(), f)?;
// 			},
// 		Node::Leaf(entries) =>
// 			for value in entries.into_iter() {
// 				f(value);
// 			},
// 	}

// 	// result
// 	Ok(())
// }

pub fn node_reader<'a, T, S: Storage>(storage: &'a S, cid: &'a Cid) -> impl Iterator<Item = anyhow::Result<T>> + 'a
where
	T: Clone + DeserializeOwned + 'static,
{
	NodeIterator::new(storage, cid)
}

struct NodeIterator<'a, T, S>
where
	T: Clone + DeserializeOwned,
{
	storage: &'a S,
	stack: VecDeque<Cid>,
	entries: VecDeque<T>,
}
impl<'a, T, S> NodeIterator<'a, T, S>
where
	T: Clone + DeserializeOwned,
	S: Storage,
{
	pub fn new(storage: &'a S, cid: &Cid) -> Self {
		let mut stack = VecDeque::new();
		stack.push_front(*cid);
		Self { storage, stack, entries: Default::default() }
	}
}
impl<'a, T, S> Iterator for NodeIterator<'a, T, S>
where
	T: Clone + DeserializeOwned,
	S: Storage,
{
	type Item = anyhow::Result<T>;

	fn next(&mut self) -> Option<Self::Item> {
		// read node
		while self.entries.is_empty() && !self.stack.is_empty() {
			if let Some(next_cid) = self.stack.pop_front() {
				let node = match read_node(self.storage, &next_cid) {
					Ok(n) => n,
					Err(e) => return Some(Err(e.into())),
				};
				match node {
					Node::Node(links) => {
						self.stack.extend(links.into_iter().map(|link| -> Cid { link.into() }));
					},
					Node::Leaf(entries) => self.entries = entries.into(),
				}
			}
		}

		// read
		self.entries.pop_front().map(|entry| Ok(entry))
	}
}

fn read_node<T: Clone + DeserializeOwned, S: Storage>(storage: &S, cid: &Cid) -> Result<Node<T>, StorageError> {
	// get block
	let block = storage.get(MultiCodec::with_dag_cbor(cid)?)?;

	// get node
	let node: Node<T> = BlockSerializer::new()
		.deserialize(&block)
		.map_err(|e| StorageError::InvalidArgument(e.into()))?;

	// result
	Ok(node)
}

// enum NodeIteratorState<'a, T>
// where
// 	T: Clone + DeserializeOwned,
// {
// 	Start(&'a dyn Storage),
// 	Node(&'a dyn Storage, Node<T>),
// 	End,
// }

// struct NodeIterator<'a, T>
// where
// 	T: Clone + DeserializeOwned,
// {
// 	state: NodeIteratorState<'a, T>,
// }

// impl<'a, T> Iterator for NodeIterator<'a, T>
// where
// 	T: Clone + DeserializeOwned,
// {
// 	type Item = anyhow::Result<T>;

// 	fn next(&mut self) -> Option<Self::Item> {
// 		match self.state {
// 			NodeIteratorState::Start(storage) => {
// 				// get block
// 				let block = storage.get(cid)?;
// 				if block.cid().codec() != Into::<u64>::into(DagCborCodec) {
// 					return Err(StorageError::InvalidArgument)?
// 				}

// 				// get node
// 				let node: Node<T> =
// 					from_cbor(block.data()).map_err(|_| StorageError::InvalidArgument)?;

// 			},
// 			NodeIteratorState::Node(storage, node) => todo!(),
// 			NodeIteratorState::End => None,
// 		}
// 	}
// }
