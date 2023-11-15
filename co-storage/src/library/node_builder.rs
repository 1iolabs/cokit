use crate::{library::to_serialized_block::to_serialized_block, types::storage::StorageError};
use co_primitives::Link;
use libipld::{Block, DefaultParams};
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
			NodeBuilderError::Encoding => StorageError::Internal,
		}
	}
}

/// Create a balances merkler tree of Node blocks.
///
/// Note: This implementation requires the data to fit into memory.
pub struct NodeBuilder<T>
where
	T: Clone,
{
	// Current Items.
	items: Vec<T>,

	/// Computed leaf blocks to store.
	blocks: Vec<Block<DefaultParams>>,

	/// Max children for each block.
	max_children: usize,
}
impl<T> NodeBuilder<T>
where
	T: Clone + Serialize,
{
	pub fn new(max_children: usize) -> Self {
		Self { items: Vec::new(), blocks: Vec::new(), max_children }
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
		let block = Self::serialize_node(&leaf)?;
		self.blocks.push(block);
		Ok(())
	}

	/// Convert builder into blocks.
	pub fn into_blocks(mut self) -> Result<Vec<Block<DefaultParams>>, NodeBuilderError> {
		// flush
		if !self.items.is_empty() {
			self.flush()?;
		}

		// result
		Self::create_balanced_links(self.blocks, self.max_children)
	}

	/// Serialize node into Block.
	fn serialize_node(node: &Node<T>) -> Result<Block<DefaultParams>, NodeBuilderError> {
		to_serialized_block(node, Default::default()).map_err(|_| NodeBuilderError::Encoding)
	}

	/// Create balanced links for all blocks.
	/// The first block in the result is the root block.
	fn create_balanced_links(
		mut blocks: Vec<Block<DefaultParams>>,
		max_children: usize,
	) -> Result<Vec<Block<DefaultParams>>, NodeBuilderError> {
		// create link blocks (all levels)
		let mut link_blocks = match blocks.len() {
			// no links needed
			0 | 1 => vec![],
			// create link nodes
			_ => {
				let level_link_blocks: Result<Vec<Block<DefaultParams>>, NodeBuilderError> = blocks
					.as_slice()
					.chunks(max_children)
					.map(|chunk| -> Node<T> {
						Node::Node(chunk.iter().map(|block| block.cid().clone().into()).collect())
					})
					.map(|node| Self::serialize_node(&node))
					.collect();
				Self::create_balanced_links(level_link_blocks?, max_children)?
			},
		};

		// append leaf blocks
		link_blocks.append(&mut blocks);

		// result
		Ok(link_blocks)
	}
}
impl<T> Default for NodeBuilder<T>
where
	T: Clone + Serialize,
{
	fn default() -> Self {
		Self::new(174)
	}
}

#[cfg(test)]
mod tests {
	use crate::library::node_builder::{Node, NodeBuilder};

	#[test]
	fn into_blocks() {
		// build
		let mut builder = NodeBuilder::<u8>::new(2);
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
		assert_eq!(7, blocks.len());
		insta::assert_debug_snapshot!(blocks);
	}

	#[test]
	fn roundtrip() {
		// build
		let mut builder = NodeBuilder::<u8>::new(2);
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
