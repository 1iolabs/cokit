use crate::{Block, BlockSerializer, DefaultParams, Link, StoreParams};
use cid::Cid;
use serde::{Deserialize, Serialize};
use std::mem::{swap, take};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node<T> {
	#[serde(rename = "n")]
	Node(Vec<Link<Self>>),
	#[serde(rename = "l")]
	Leaf(Vec<T>),
}
impl<T> Default for Node<T> {
	fn default() -> Self {
		Node::Leaf(vec![])
	}
}

#[derive(Debug, thiserror::Error)]
pub enum NodeBuilderError {
	#[error("Encoding failed")]
	Encoding,
}

pub trait NodeSerializer<T, P>
where
	T: Clone,
	P: StoreParams,
{
	fn serialize(&self, node: &Node<T>) -> Result<Block<P>, NodeBuilderError>;
}

pub struct DefaultNodeSerializer {}
impl Default for DefaultNodeSerializer {
	fn default() -> Self {
		Self::new()
	}
}
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
pub struct NodeBuilder<T, P = DefaultParams, S = DefaultNodeSerializer>
where
	T: Clone,
	P: StoreParams,
	S: NodeSerializer<T, P>,
{
	// Current Items.
	items: Vec<T>,

	/// Leaf block references.
	blocks: Vec<Cid>,

	/// Computed leaf blocks to store.
	pending_blocks: Vec<Block<P>>,

	/// Max children for each block.
	max_children: usize,

	/// Serializer.
	serializer: S,
}
impl<T, P, S> NodeBuilder<T, P, S>
where
	T: Clone + Serialize,
	P: StoreParams,
	S: NodeSerializer<T, P>,
{
	pub fn new(max_children: usize, serializer: S) -> Self {
		Self { items: Vec::new(), blocks: Vec::new(), pending_blocks: Vec::new(), max_children, serializer }
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

	/// Take blocks from builder that already has been created.
	pub fn take_blocks(&mut self) -> impl Iterator<Item = Block<P>> {
		let mut blocks = Vec::new();
		swap(&mut self.pending_blocks, &mut blocks);
		blocks.into_iter()
	}

	/// Flush items into new leaf block.
	fn flush(&mut self) -> Result<(), NodeBuilderError> {
		let leaf = Node::Leaf(take(&mut self.items));
		let block = self.serializer.serialize(&leaf)?;
		self.blocks.push(*block.cid());
		self.pending_blocks.push(block);
		Ok(())
	}

	/// Convert builder into blocks.
	/// All blocks that are not yet taken using [`NodeBuilder::take_blocks`] are returned.
	pub fn into_blocks(mut self) -> Result<(Option<Cid>, Vec<Block<P>>), NodeBuilderError> {
		// flush
		if !self.items.is_empty() {
			self.flush()?;
		}

		// result
		Ok((
			Self::create_balanced_links(&self.serializer, self.blocks, self.max_children, &mut self.pending_blocks)?,
			self.pending_blocks,
		))
	}

	/// Create balanced links for all blocks.
	/// Returns the [`Cid`] of the root if not empty.
	fn create_balanced_links(
		serializer: &S,
		blocks: Vec<Cid>,
		max_children: usize,
		pending_blocks: &mut Vec<Block<P>>,
	) -> Result<Option<Cid>, NodeBuilderError> {
		// create link blocks (all levels)
		Ok(match blocks.len() {
			// no links needed
			0 => None,
			1 => blocks.into_iter().next(),
			// create link nodes
			_ => {
				let mut level_link_blocks = blocks
					.as_slice()
					.chunks(max_children)
					.map(|chunk| -> Node<T> { Node::Node(chunk.iter().map(|leaf| leaf.into()).collect()) })
					.map(|node| serializer.serialize(&node))
					.collect::<Result<Vec<Block<P>>, NodeBuilderError>>()?;
				let level_links = level_link_blocks.iter().map(|block| *block.cid()).collect::<Vec<Cid>>();

				// store created link blocks
				pending_blocks.append(&mut level_link_blocks);

				// create next level
				Self::create_balanced_links(serializer, level_links, max_children, pending_blocks)?
			},
		})
	}
}
impl<T, P> Default for NodeBuilder<T, P, DefaultNodeSerializer>
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
		let (_root, blocks) = builder.into_blocks().unwrap();
		assert_eq!(blocks.len(), 7);
		insta::assert_debug_snapshot!(blocks);
	}

	#[test]
	fn single() {
		// build
		let mut builder = NodeBuilder::<u8>::new(2, DefaultNodeSerializer::new());
		builder.push(1).unwrap();

		// blocks
		let (_root, blocks) = builder.into_blocks().unwrap();
		assert_eq!(blocks.len(), 1);
	}

	#[test]
	fn empty() {
		// build
		let builder = NodeBuilder::<u8>::new(2, DefaultNodeSerializer::new());

		// blocks
		let (_root, blocks) = builder.into_blocks().unwrap();
		assert_eq!(blocks.len(), 0);
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
		builder.push(7).unwrap();
		builder.push(8).unwrap();

		// blocks
		let (_root, blocks) = builder.into_blocks().unwrap();

		// nodes
		let nodes: Vec<Node<u8>> = blocks
			.iter()
			.map(|block| serde_ipld_dagcbor::from_slice::<Node<u8>>(block.data()).unwrap())
			.collect();
		insta::assert_yaml_snapshot!(nodes);
	}
}
