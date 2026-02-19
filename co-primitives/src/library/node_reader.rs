// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{from_cbor, MultiCodec, Node, Storage};
use cid::Cid;
use serde::de::DeserializeOwned;
use std::collections::VecDeque;

#[derive(Debug, thiserror::Error)]
pub enum NodeReaderError {
	#[error("Invalid argument")]
	InvalidArgument(#[source] anyhow::Error),

	#[error("Decode failed")]
	Decode(#[source] anyhow::Error),
}

pub fn node_reader<T>(storage: &dyn Storage, cid: Option<Cid>) -> impl Iterator<Item = Result<T, NodeReaderError>> + '_
where
	T: Clone + DeserializeOwned + 'static,
{
	NodeIterator::new(storage, cid)
}

pub struct NodeIterator<'a, T>
where
	T: 'a + Clone + DeserializeOwned,
{
	storage: &'a dyn Storage,
	stack: VecDeque<Cid>,
	entries: VecDeque<T>,
}

impl<'a, T> NodeIterator<'a, T>
where
	T: Clone + DeserializeOwned,
{
	pub fn new(storage: &'a dyn Storage, cid: Option<Cid>) -> Self {
		let mut stack = VecDeque::new();
		if let Some(cid) = cid {
			stack.push_front(cid);
		}
		Self { storage, stack, entries: Default::default() }
	}
}

impl<'a, T> Iterator for NodeIterator<'a, T>
where
	T: 'a + Clone + DeserializeOwned,
{
	type Item = Result<T, NodeReaderError>;

	fn next(&mut self) -> Option<Self::Item> {
		// read node
		while self.entries.is_empty() && !self.stack.is_empty() {
			if let Some(next_cid) = self.stack.pop_front() {
				let node = match read_node(self.storage, &next_cid) {
					Ok(n) => n,
					Err(e) => return Some(Err(e)),
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

fn read_node<T: Clone + DeserializeOwned>(storage: &dyn Storage, cid: &Cid) -> Result<Node<T>, NodeReaderError> {
	// get block
	let block = storage.get(MultiCodec::with_cbor(cid).map_err(|err| NodeReaderError::InvalidArgument(err.into()))?);

	// get node
	let node: Node<T> = from_cbor(block.data()).map_err(|e| NodeReaderError::Decode(e.into()))?;

	// result
	Ok(node)
}
