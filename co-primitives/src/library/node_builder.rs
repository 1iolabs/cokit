use super::dag_cbor_size_serializer::DagCborSizeSerializer;
use crate::{Block, BlockSerializer, DefaultParams, Link, OptionLink, StoreParams};
use cid::Cid;
use either::Either;
use serde::{Deserialize, Serialize};
use std::{
	marker::PhantomData,
	mem::{swap, take},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node<T> {
	#[serde(rename = "n")]
	Node(Vec<Link<Self>>),
	#[serde(rename = "l")]
	Leaf(Vec<T>),
}
impl<T> Node<T> {
	pub fn is_empty(&self) -> bool {
		match self {
			Node::Node(items) => items.is_empty(),
			Node::Leaf(links) => links.is_empty(),
		}
	}
}
impl<T> Default for Node<T> {
	fn default() -> Self {
		Node::Leaf(vec![])
	}
}
impl<T> NodeReader<T> for Node<T> {
	type Filter = ();

	fn read(self, _: &Self::Filter) -> Either<Vec<Cid>, Vec<T>> {
		match self {
			Node::Node(links) => Either::Left(links.into_iter().map(Into::into).collect()),
			Node::Leaf(items) => Either::Right(items),
		}
	}
}

pub trait NodeReader<T> {
	type Filter: Default;

	fn read(self, filter: &Self::Filter) -> Either<Vec<Cid>, Vec<T>>;
}

#[derive(Debug, thiserror::Error)]
pub enum NodeBuilderError {
	#[error("Encoding failed")]
	Encoding(#[source] anyhow::Error),

	#[error("Invalid argument")]
	InvalidArgument(#[source] anyhow::Error),
}

pub trait NodeSerializer<N, T> {
	fn nodes(&mut self, nodes: Vec<Link<N>>) -> Result<N, NodeBuilderError>;

	fn leaf(&mut self, entries: Vec<T>) -> Result<N, NodeBuilderError>;

	fn serialize(&mut self, max_block_size: usize, node: N) -> Result<Block, NodeBuilderError>;

	fn item_size_hint(&self, item: &T) -> Option<usize> {
		let _item = item;
		None
	}
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
impl<T> NodeSerializer<Node<T>, T> for DefaultNodeSerializer
where
	T: Serialize,
{
	fn nodes(&mut self, nodes: Vec<Link<Node<T>>>) -> Result<Node<T>, NodeBuilderError> {
		Ok(Node::Node(nodes))
	}

	fn leaf(&mut self, entries: Vec<T>) -> Result<Node<T>, NodeBuilderError> {
		Ok(Node::Leaf(entries))
	}

	fn serialize(&mut self, max_block_size: usize, node: Node<T>) -> Result<Block, NodeBuilderError> {
		BlockSerializer::new()
			.with_max_block_size(max_block_size)
			.serialize(&node)
			.map_err(|err| NodeBuilderError::Encoding(err.into()))
	}

	fn item_size_hint(&self, item: &T) -> Option<usize> {
		let mut serializer = DagCborSizeSerializer::new();
		item.serialize(&mut serializer).ok()?;
		Some(serializer.size)
	}
}

/// Create a balances merkle tree of Node blocks.
///
/// Note: This implementation requires the data to fit into memory.
pub struct NodeBuilder<T, N = Node<T>, S = DefaultNodeSerializer>
where
	T: Clone,
	S: NodeSerializer<N, T>,
{
	_node: PhantomData<N>,

	// Current Items.
	items: Vec<T>,
	items_size: usize,
	items_size_max: usize,

	/// Block leaf references.
	blocks: Vec<Link<N>>,

	/// Computed leaf blocks to store.
	pending_blocks: Vec<Block>,

	/// Max children for each block.
	max_children: usize,
	max_block_size: usize,

	/// Serializer.
	serializer: S,
}
impl<T, N, S> NodeBuilder<T, N, S>
where
	T: Clone + Serialize,
	S: NodeSerializer<N, T>,
{
	pub fn new(max_block_size: usize, max_children: usize, serializer: S) -> Self {
		Self {
			_node: Default::default(),
			items: Vec::new(),
			items_size: 0,
			items_size_max: max_block_size * 3 / 4,
			max_block_size,
			blocks: Vec::new(),
			pending_blocks: Vec::new(),
			max_children,
			serializer,
		}
	}

	pub fn with_items_size_max(mut self, items_size_max: usize) -> Self {
		debug_assert!(items_size_max > 0);
		self.items_size_max = items_size_max;
		self
	}

	pub fn push(&mut self, item: T) -> Result<(), NodeBuilderError> {
		// size
		let item_size = self.serializer.item_size_hint(&item).unwrap_or(0);
		if self.items_size + item_size >= self.items_size_max {
			self.flush()?;
		}

		// push item
		self.items_size += item_size;
		self.items.push(item);

		// full?
		if self.items.len() >= self.max_children || self.items_size >= self.items_size_max {
			self.flush()?;
		}

		// done
		Ok(())
	}

	pub fn extend(&mut self, items: impl IntoIterator<Item = T>) -> Result<(), NodeBuilderError> {
		for item in items {
			self.push(item)?;
		}
		Ok(())
	}

	/// Take blocks from builder that already has been created.
	pub fn take_blocks(&mut self) -> impl Iterator<Item = Block> {
		let mut blocks = Vec::new();
		swap(&mut self.pending_blocks, &mut blocks);
		blocks.into_iter()
	}

	/// Flush items into new leaf block.
	fn flush(&mut self) -> Result<(), NodeBuilderError> {
		let leaf = self.serializer.leaf(take(&mut self.items))?;
		let block = self.serializer.serialize(self.max_block_size, leaf)?;
		self.items_size = 0;
		self.blocks.push(block.cid().into());
		self.pending_blocks.push(block);
		Ok(())
	}

	/// Flush blocks into new node block.
	fn flush_level(&mut self) -> Result<(), NodeBuilderError> {
		let mut level_link_blocks = self
			.blocks
			.as_slice()
			.chunks(self.max_children)
			.map(|chunk| {
				let node = self.serializer.nodes(chunk.to_vec())?;
				let block = self.serializer.serialize(self.max_block_size, node)?;
				Ok(block)
			})
			.collect::<Result<Vec<Block>, NodeBuilderError>>()?;
		let level_links = level_link_blocks
			.iter()
			.map(|block| block.cid().into())
			.collect::<Vec<Link<N>>>();

		// store created link blocks
		self.pending_blocks.append(&mut level_link_blocks);

		// apply level
		self.blocks = level_links;

		// result
		Ok(())
	}

	/// Convert builder into blocks.
	/// All blocks that are not yet taken using [`NodeBuilder::take_blocks`] are returned.
	pub fn into_blocks(mut self) -> Result<(OptionLink<N>, Vec<Block>), NodeBuilderError> {
		// empty?
		if self.items.is_empty() && self.blocks.is_empty() {
			return Ok((Default::default(), Default::default()));
		}

		// node
		let (node, mut blocks) = self.take_node()?;
		let root = self.serializer.serialize(self.max_block_size, node)?;
		let root_link = root.cid().into();
		blocks.push(root);
		Ok((root_link, blocks))
	}

	/// Convert builder into a node and blocks if needed.
	/// All blocks that are not yet taken using [`NodeBuilder::take_blocks`] are returned.
	/// The root node is returned directly and not put into a block.
	pub fn into_node(mut self) -> Result<(N, Vec<Block>), NodeBuilderError> {
		self.take_node()
	}

	/// Take node and blocks. The serializer will be left empty.
	fn take_node(&mut self) -> Result<(N, Vec<Block>), NodeBuilderError> {
		// return a leaf if have no full blocks
		if self.blocks.is_empty() {
			let node = self.serializer.leaf(take(&mut self.items))?;
			return Ok((node, Default::default()));
		}

		// flush
		if !self.items.is_empty() {
			self.flush()?;
		}
		while self.blocks.len() > self.max_children {
			self.flush_level()?;
		}

		// node
		let node = self.serializer.nodes(take(&mut self.blocks))?;

		// result
		Ok((node, take(&mut self.pending_blocks)))
	}
}
impl<T> Default for NodeBuilder<T, Node<T>, DefaultNodeSerializer>
where
	T: Clone + Serialize,
{
	fn default() -> Self {
		Self::new(DefaultParams::MAX_BLOCK_SIZE, 174, DefaultNodeSerializer::new())
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		library::node_builder::{DefaultNodeSerializer, Node, NodeBuilder},
		DefaultParams, StoreParams,
	};
	use std::iter::repeat_n;

	#[test]
	fn into_blocks() {
		// build
		let mut builder = NodeBuilder::<u8>::new(DefaultParams::MAX_BLOCK_SIZE, 2, DefaultNodeSerializer::new());
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
		let mut builder = NodeBuilder::<u8>::new(DefaultParams::MAX_BLOCK_SIZE, 2, DefaultNodeSerializer::new());
		builder.push(1).unwrap();

		// blocks
		let (_root, blocks) = builder.into_blocks().unwrap();
		assert_eq!(blocks.len(), 1);
	}

	#[test]
	fn empty() {
		// build
		let builder = NodeBuilder::<u8>::new(DefaultParams::MAX_BLOCK_SIZE, 2, DefaultNodeSerializer::new());

		// blocks
		let (_root, blocks) = builder.into_blocks().unwrap();
		assert_eq!(blocks.len(), 0);
	}

	#[test]
	fn roundtrip() {
		// build
		let mut builder = NodeBuilder::<u8>::new(DefaultParams::MAX_BLOCK_SIZE, 2, DefaultNodeSerializer::new());
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

	#[test]
	fn big_blocks() {
		// build
		let mut builder = NodeBuilder::<Vec<u8>>::new(DefaultParams::MAX_BLOCK_SIZE, 174, DefaultNodeSerializer::new());
		let block_size = DefaultParams::MAX_BLOCK_SIZE / 10;
		for _ in 0..11 {
			builder.push(repeat_n(0u8, block_size).collect()).unwrap();
		}

		// blocks
		let (_root, blocks) = builder.into_blocks().unwrap();
		assert_eq!(blocks.len(), 3);
	}
}
