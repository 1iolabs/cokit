use crate::{BlockLinks, BlockStorage, StorageError};
use cid::Cid;
use futures::Stream;
use std::collections::{BinaryHeap, HashMap, VecDeque};

/// Find added/removed references in blocks.
/// If a [`Cid`] is referenced multiple times it will also returnes multiple times.
/// This diff works recursively - all added/removed [`Cid`] at any depth will be returned.
/// However there is a look-ahead depth limit, how many nodes are looked down to reuse references (defaults to `1`).
pub fn block_diff<S>(
	storage: S,
	prev: Option<Cid>,
	next: Cid,
	block_links: BlockLinks,
	look_ahead_depth: Option<u8>,
) -> impl Stream<Item = Result<BlockDiff, StorageError>>
where
	S: BlockStorage + Clone + 'static,
{
	async_stream::try_stream! {
		let mut stack = VecDeque::new();
		stack.push_back(next);

		// collect prev (one level)
		let mut prev_links: HashMap<Cid, BinaryHeap<ReferenceDepth>> = HashMap::new();
		if let Some(prev) = prev {
			prev_links.insert(prev, BinaryHeap::from([ReferenceDepth::Deep]));
		}

		// walk next
		while let Some(next) = stack.pop_front() {
			// try to reuse a reference
			let mut allow_resolve_next_levels = look_ahead_depth.unwrap_or(1);
			loop {
				match pop_reference(&mut prev_links, &next) {
					// found deep reference -> (re-)use it
					Some(ReferenceDepth::Deep) | Some(ReferenceDepth::DeepNotFound) => {
						// result.push(Diff::Reuse(*next));
					},
					// found shallow reference -> (re-)use it and try to reuse its links
					Some(ReferenceDepth::Shallow) => {
						// walk children
						stack.extend(extract_links_children(&storage, &next, &block_links).await?);
						// result.push(Diff::Reuse(*next));
					},
					// no previous reference found
					None => {
						// resolve one level of deep references
						make_shallow(&storage, &block_links, &mut prev_links).await?;

						// retry with next level resolved
						if allow_resolve_next_levels > 0 {
							allow_resolve_next_levels -= 1;
							continue;
						}

						// walk children
						stack.extend(extract_links_children(&storage, &next, &block_links).await?);

						// add as added
						yield BlockDiff::Added(next);
					},
				}
				break;
			}
		}

		// removed
		for (cid, references) in prev_links.into_iter() {
			for reference in references {
				match reference {
					ReferenceDepth::Deep => {
						yield BlockDiff::Removed(cid);
						for await descendant in extract_links_descendants(&storage, &block_links, cid) {
							match descendant {
								Ok(descendant) => {
									yield BlockDiff::Removed(descendant);
								},
								Err(_err) => {
									// ignore not found (others?)
								},
							}
						}
					},
					ReferenceDepth::Shallow => {
						yield BlockDiff::Removed(cid);
					},
					ReferenceDepth::DeepNotFound => {
						// ignore not found
					},
				}
			}
		}
	}
}

/// Pop a ReferenceDepth for next, if one.
fn pop_reference(prev_links: &mut HashMap<Cid, BinaryHeap<ReferenceDepth>>, next: &Cid) -> Option<ReferenceDepth> {
	// pop previous
	let reference = match prev_links.get_mut(&next) {
		Some(prev) => prev.pop(),
		None => None,
	};

	// remove empty keys
	if reference.is_some() {
		if let Some(references) = prev_links.get(&next) {
			if references.is_empty() {
				prev_links.remove(&next);
			}
		}
	}

	// result
	reference
}

/// Make all deep references shallow.
async fn make_shallow<S>(
	storage: &S,
	links: &BlockLinks,
	prev_links: &mut HashMap<Cid, BinaryHeap<ReferenceDepth>>,
) -> Result<(), StorageError>
where
	S: BlockStorage + Clone + 'static,
{
	let deep_referencs: Vec<Cid> = prev_links
		.iter()
		.filter_map(|(cid, references)| match references.peek() {
			Some(ReferenceDepth::Deep) => Some(*cid),
			_ => None,
		})
		.collect();
	for cid in deep_referencs {
		make_reference_shallow(storage, links, prev_links, cid).await?;
	}
	Ok(())
}

/// Make a single deep reference shallow.
async fn make_reference_shallow<S>(
	storage: &S,
	block_links: &BlockLinks,
	prev_links: &mut HashMap<Cid, BinaryHeap<ReferenceDepth>>,
	cid: Cid,
) -> Result<(), StorageError>
where
	S: BlockStorage + Clone + 'static,
{
	// replace one deep reference with shallow
	let links = if let Some(prev) = prev_links.get_mut(&cid) {
		if let Some(mut reference) = prev.peek_mut() {
			match *reference {
				ReferenceDepth::Deep => match extract_links_children(storage, &cid, &block_links).await {
					Ok(links) => {
						*reference = ReferenceDepth::Shallow;
						Some(links)
					},
					Err(StorageError::NotFound(_, _)) => {
						*reference = ReferenceDepth::DeepNotFound;
						None
					},
					Err(err) => return Err(err),
				},
				_ => None,
			}
		} else {
			None
		}
	} else {
		None
	};

	// add new resolved links
	if let Some(links) = links {
		for link in links {
			prev_links.entry(link).or_insert(Default::default()).push(ReferenceDepth::Deep);
		}
	}

	Ok(())
}

/// Extract children links from reference.
async fn extract_links_children<S>(storage: &S, reference: &Cid, links: &BlockLinks) -> Result<Vec<Cid>, StorageError>
where
	S: BlockStorage + Clone + 'static,
{
	let block = storage.get(reference).await?;
	let result = links.links(&block).map_err(|err| StorageError::Internal(err))?.collect();
	Ok(result)
}

/// Extract descendant links from reference.
/// Does not stop after encountering an error.
fn extract_links_descendants<'a, S>(
	storage: &'a S,
	block_links: &'a BlockLinks,
	reference: Cid,
) -> impl Stream<Item = Result<Cid, StorageError>> + use<'a, S>
where
	S: BlockStorage + Clone + 'static,
{
	async_stream::stream! {
		let mut stack = VecDeque::new();
		stack.push_back(reference);
		while let Some(reference) = stack.pop_front() {
			let block = match storage.get(&reference).await {
				Ok(block) => block,
				Err(err) => {
					yield Err(err);
					continue;
				}
			};
			let links = match block_links.links(&block) {
				Ok(links) => links,
				Err(err) => {
					yield Err(StorageError::Internal(err));
					continue;
				},
			};
			for link in links {
				yield Ok(link);
				stack.push_back(link);
			}
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ReferenceDepth {
	/// Reference is deep (all descendants).
	Deep,

	/// Reference is shallow (no descendants).
	Shallow,

	/// Reference not found. Same as deep but will not be resolved to shallow again (all descendants).
	DeepNotFound,
}

#[derive(Debug, PartialEq)]
pub enum BlockDiff {
	/// Item has been added.
	Added(Cid),

	/// Item has been removed.
	Removed(Cid),
}

#[cfg(test)]
mod tests {
	use crate::{
		library::{
			block_diff::{block_diff, BlockDiff},
			test::TestStorage,
		},
		BlockStorageExt,
	};
	use cid::Cid;
	use futures::TryStreamExt;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Serialize, Deserialize, PartialEq)]
	struct Node {
		id: u32,
		nodes: Vec<Cid>,
	}

	/// Replace a node and its root.
	///
	/// From:
	/// ```mermaid
	/// flowchart TD
	///   5 --> 1
	///   5 --> 2
	///   6 --> 3
	///   6 --> 4
	///   7 --> 5
	///   7 --> 6
	/// ```
	///
	/// To:
	/// ```mermaid
	/// flowchart TD
	///   5' --> 1
	///   5' --> 2
	///   6 --> 3
	///   6 --> 4
	///   7' --> 5'
	///   7' --> 6
	/// ```
	#[tokio::test]
	async fn test_one_level() {
		let storage = TestStorage::default();

		// create
		let node1 = storage.set_serialized(&Node { id: 1, nodes: vec![] }).await.unwrap();
		let node2 = storage.set_serialized(&Node { id: 2, nodes: vec![] }).await.unwrap();
		let node3 = storage.set_serialized(&Node { id: 3, nodes: vec![] }).await.unwrap();
		let node4 = storage.set_serialized(&Node { id: 4, nodes: vec![] }).await.unwrap();
		let node5 = storage
			.set_serialized(&Node { id: 5, nodes: vec![node1, node2] })
			.await
			.unwrap();
		let node6 = storage
			.set_serialized(&Node { id: 6, nodes: vec![node3, node4] })
			.await
			.unwrap();
		let node7 = storage
			.set_serialized(&Node { id: 7, nodes: vec![node5, node6] })
			.await
			.unwrap();

		// update
		let node5_change = storage
			.set_serialized(&Node { id: 50, nodes: vec![node1, node2] })
			.await
			.unwrap();
		let node7_change = storage
			.set_serialized(&Node { id: 70, nodes: vec![node5_change, node6] })
			.await
			.unwrap();

		// diff
		let diff = block_diff(storage, Some(node7), node7_change, Default::default(), Default::default())
			.try_collect::<Vec<BlockDiff>>()
			.await
			.unwrap();
		assert_eq!(diff.len(), 4);
		assert!(diff.contains(&BlockDiff::Added(node7_change)));
		assert!(diff.contains(&BlockDiff::Added(node5_change)));
		assert!(diff.contains(&BlockDiff::Removed(node7)));
		assert!(diff.contains(&BlockDiff::Removed(node5)));
	}

	/// Replace a node and its root.
	///
	/// From:
	/// ```mermaid
	/// flowchart TD
	///   5 --> 1
	///   5 --> 2
	///   6 --> 3
	///   6 --> 4
	///   7 --> 5
	///   7 --> 6
	/// ```
	///
	/// To:
	/// ```mermaid
	/// flowchart TD
	///   5 --> 1
	///   5 --> 2
	///   6 --> 3
	///   6 --> 4
	///   7 --> 5
	///   7 --> 6
	///   8' --> 7
	/// ```
	#[tokio::test]
	async fn test_reparent_root() {
		let storage = TestStorage::default();

		// create
		let node1 = storage.set_serialized(&Node { id: 1, nodes: vec![] }).await.unwrap();
		let node2 = storage.set_serialized(&Node { id: 2, nodes: vec![] }).await.unwrap();
		let node3 = storage.set_serialized(&Node { id: 3, nodes: vec![] }).await.unwrap();
		let node4 = storage.set_serialized(&Node { id: 4, nodes: vec![] }).await.unwrap();
		let node5 = storage
			.set_serialized(&Node { id: 5, nodes: vec![node1, node2] })
			.await
			.unwrap();
		let node6 = storage
			.set_serialized(&Node { id: 6, nodes: vec![node3, node4] })
			.await
			.unwrap();
		let node7 = storage
			.set_serialized(&Node { id: 7, nodes: vec![node5, node6] })
			.await
			.unwrap();

		// update
		let node8_change = storage.set_serialized(&Node { id: 8, nodes: vec![node7] }).await.unwrap();

		// diff
		let diff = block_diff(storage, Some(node7), node8_change, Default::default(), Default::default())
			.try_collect::<Vec<BlockDiff>>()
			.await
			.unwrap();
		assert_eq!(diff.len(), 1);
		assert!(diff.contains(&BlockDiff::Added(node8_change)));
	}
}
