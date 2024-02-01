use crate::types::storage::StorageError;
use co_primitives::{BlockSerializer, Link};
use libipld::{store::StoreParams, Block, DefaultParams};
use serde::{Deserialize, Serialize};
use std::mem::take;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node<T>
where
	T: Clone,
{
	#[serde(rename = "n")]
	Node(Vec<Link<Self>>),
	#[serde(rename = "l")]
	Leaf(Vec<T>),
}

#[derive(Debug, thiserror::Error)]
pub enum NodeBuilderError {
	#[error("Encoding failed")]
	Encoding,
}
impl Into<StorageError> for NodeBuilderError {
	fn into(self) -> StorageError {
		match self {
			NodeBuilderError::Encoding => StorageError::Internal(self.into()),
		}
	}
}

pub trait NodeSerializer<T, P>
where
	T: Clone,
	P: StoreParams,
{
	fn serialize(&self, node: &Node<T>) -> Result<Block<P>, NodeBuilderError>;
}

pub struct DefaultNodeSerializer {}
impl DefaultNodeSerializer {
	pub fn new() -> Self {
		Self {}
	}
}
impl<T, P> NodeSerializer<T, P> for DefaultNodeSerializer
where
	T: Clone + Serialize,
	P: StoreParams,
{
	fn serialize(&self, node: &Node<T>) -> Result<Block<P>, NodeBuilderError> {
		BlockSerializer::new().serialize(node).map_err(|_| NodeBuilderError::Encoding)
	}
}

/// Create a balances merkler tree of Node blocks.
///
/// Note: This implementation requires the data to fit into memory.
pub struct NodeBuilder<T, S = DefaultNodeSerializer, P = DefaultParams>
where
	T: Clone,
	S: NodeSerializer<T, P>,
	P: StoreParams,
{
	// Current Items.
	items: Vec<T>,

	/// Computed leaf blocks to store.
	blocks: Vec<Block<P>>,

	/// Max children for each block.
	max_children: usize,

	/// Serializer.
	serializer: S,
}
impl<T, S, P> NodeBuilder<T, S, P>
where
	T: Clone + Serialize,
	S: NodeSerializer<T, P>,
	P: StoreParams,
{
	pub fn new(max_children: usize, serializer: S) -> Self {
		Self { items: Vec::new(), blocks: Vec::new(), max_children, serializer }
	}

	pub fn push(&mut self, item: T) -> Result<(), NodeBuilderError> {
		// push item
		self.items.push(item);

		// full?
		if self.items.len() >= self.max_children {
			self.flush()?;
		}

		// done
		Ok(())
	}

	/// Flush items into new leaf block.
	fn flush(&mut self) -> Result<(), NodeBuilderError> {
		let leaf = Node::Leaf(take(&mut self.items));
		let block = self.serializer.serialize(&leaf)?;
		self.blocks.push(block);
		Ok(())
	}

	/// Convert builder into blocks.
	pub fn into_blocks(mut self) -> Result<Vec<Block<P>>, NodeBuilderError> {
		// flush
		if !self.items.is_empty() {
			self.flush()?;
		}

		// result
		Self::create_balanced_links(&self.serializer, self.blocks, self.max_children)
	}

	/// Create balanced links for all blocks.
	/// The first block in the result is the root block.
	fn create_balanced_links(
		serializer: &S,
		mut blocks: Vec<Block<P>>,
		max_children: usize,
	) -> Result<Vec<Block<P>>, NodeBuilderError> {
		// create link blocks (all levels)
		let mut link_blocks = match blocks.len() {
			// no links needed
			0 | 1 => vec![],
			// create link nodes
			_ => {
				let level_link_blocks: Result<Vec<Block<P>>, NodeBuilderError> = blocks
					.as_slice()
					.chunks(max_children)
					.map(|chunk| -> Node<T> {
						Node::Node(chunk.iter().map(|block| block.cid().clone().into()).collect())
					})
					.map(|node| serializer.serialize(&node))
					.collect();
				Self::create_balanced_links(serializer, level_link_blocks?, max_children)?
			},
		};

		// append leaf blocks
		link_blocks.append(&mut blocks);

		// result
		Ok(link_blocks)
	}
}
impl<T, P> Default for NodeBuilder<T, DefaultNodeSerializer, P>
where
	T: Clone + Serialize,
	P: StoreParams,
{
	fn default() -> Self {
		Self::new(174, DefaultNodeSerializer::new())
	}
}

#[cfg(test)]
mod tests {
	use crate::library::node_builder::{DefaultNodeSerializer, Node, NodeBuilder};

	#[test]
	fn into_blocks() {
		// build
		let mut builder = NodeBuilder::<u8>::new(2, DefaultNodeSerializer::new());
		builder.push(1).unwrap();
		builder.push(2).unwrap();
		builder.push(3).unwrap();
		builder.push(4).unwrap();
		builder.push(5).unwrap();
		builder.push(6).unwrap();
		builder.push(7).unwrap();
		builder.push(8).unwrap();

		// blocks
		let blocks = builder.into_blocks().unwrap();
		assert_eq!(blocks.len(), 7);
		insta::assert_debug_snapshot!(blocks);
	}

	#[test]
	fn roundtrip() {
		// build
		let mut builder = NodeBuilder::<u8>::new(2, DefaultNodeSerializer::new());
		builder.push(1).unwrap();
		builder.push(2).unwrap();
		builder.push(3).unwrap();
		builder.push(4).unwrap();
		builder.push(5).unwrap();
		builder.push(6).unwrap();
		builder.push(8).unwrap();
		builder.push(9).unwrap();

		// blocks
		let blocks = builder.into_blocks().unwrap();

		// nodes
		let nodes: Vec<Node<u8>> = blocks
			.iter()
			.map(|block| serde_ipld_dagcbor::from_slice::<Node<u8>>(block.data()).unwrap())
			.collect();
		insta::assert_yaml_snapshot!(nodes);
	}
}
