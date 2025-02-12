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
	Encoding,

	#[error("Invalid argument")]
	InvalidArgument(#[source] anyhow::Error),
}

pub trait NodeSerializer<N, T, P>
where
	P: StoreParams,
{
	fn nodes(&mut self, nodes: Vec<Link<N>>) -> Result<N, NodeBuilderError>;
	fn leaf(&mut self, entries: Vec<T>) -> Result<N, NodeBuilderError>;
	fn serialize(&mut self, node: N) -> Result<Block<P>, NodeBuilderError>;
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
impl<T, P> NodeSerializer<Node<T>, T, P> for DefaultNodeSerializer
where
	T: Serialize,
	P: StoreParams,
{
	fn nodes(&mut self, nodes: Vec<Link<Node<T>>>) -> Result<Node<T>, NodeBuilderError> {
		Ok(Node::Node(nodes))
	}

	fn leaf(&mut self, entries: Vec<T>) -> Result<Node<T>, NodeBuilderError> {
		Ok(Node::Leaf(entries))
	}

	fn serialize(&mut self, node: Node<T>) -> Result<Block<P>, NodeBuilderError> {
		BlockSerializer::new().serialize(&node).map_err(|_| NodeBuilderError::Encoding)
	}
}

/// Create a balances merkle tree of Node blocks.
///
/// Note: This implementation requires the data to fit into memory.
pub struct NodeBuilder<T, P = DefaultParams, N = Node<T>, S = DefaultNodeSerializer>
where
	T: Clone,
	P: StoreParams,
	S: NodeSerializer<N, T, P>,
{
	_node: PhantomData<N>,

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
impl<T, P, N, S> NodeBuilder<T, P, N, S>
where
	T: Clone + Serialize,
	P: StoreParams,
	S: NodeSerializer<N, T, P>,
{
	pub fn new(max_children: usize, serializer: S) -> Self {
		Self {
			_node: Default::default(),
			items: Vec::new(),
			blocks: Vec::new(),
			pending_blocks: Vec::new(),
			max_children,
			serializer,
		}
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
		let leaf = self.serializer.leaf(take(&mut self.items))?;
		let block = self.serializer.serialize(leaf)?;
		self.blocks.push(*block.cid());
		self.pending_blocks.push(block);
		Ok(())
	}

	/// Convert builder into blocks.
	/// All blocks that are not yet taken using [`NodeBuilder::take_blocks`] are returned.
	pub fn into_blocks(mut self) -> Result<(OptionLink<N>, Vec<Block<P>>), NodeBuilderError> {
		// flush
		if !self.items.is_empty() {
			self.flush()?;
		}

		// result
		Ok((
			Self::create_balanced_links(
				&mut self.serializer,
				self.blocks,
				self.max_children,
				&mut self.pending_blocks,
			)?
			.into(),
			self.pending_blocks,
		))
	}

	/// Create balanced links for all blocks.
	/// Returns the [`Cid`] of the root if not empty.
	fn create_balanced_links(
		serializer: &mut S,
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
					.map(|chunk| {
						let node = serializer.nodes(chunk.iter().map(|leaf| leaf.into()).collect())?;
						let block = serializer.serialize(node)?;
						Ok(block)
					})
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
impl<T, P> Default for NodeBuilder<T, P, Node<T>, DefaultNodeSerializer>
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
