// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_primitives::Streamable;
use co_storage::{BlockStorage, StorageError};
use futures::{pin_mut, StreamExt};
use serde::de::DeserializeOwned;

/// Find first element in an [`DagCollectionAsyncExt`] that matches an predicate.
/// When an error is encountered it will be ignored and the search for the element continuts until there are no more
/// elements, then the first error is returned.
pub async fn find<T, N, F, S>(storage: &S, container: &N, predicate: F) -> Result<Option<T>, StorageError>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
	T: std::fmt::Debug + DeserializeOwned + Send + Sync + 'static,
	N: Streamable<S, Item = Result<T, StorageError>>,
	F: Fn(&T) -> bool,
{
	let stream = container.stream(storage.clone());
	pin_mut!(stream);
	let mut result = Ok(None);
	while let Some(item) = stream.next().await {
		match item {
			Ok(value) => {
				if predicate(&value) {
					// first value
					result = Ok(Some(value));
					break;
				}
			},
			Err(err) => {
				if result.is_ok() {
					// first error
					result = Err(err);
				}
			},
		}
	}
	result
	// NodeStream::from_node_container(storage.clone(), container)
	// 	.filter(|result| {
	// 		println!(
	// 			"filter {:?} - {:?}",
	// 			result,
	// 			match result {
	// 				Ok(value) => predicate(&value),
	// 				Err(_) => true,
	// 			}
	// 		);
	// 		ready(match result {
	// 			Ok(value) => predicate(&value),
	// 			Err(_) => true,
	// 		})
	// 	})
	// 	.take_while_incl(|result| ready(result.is_ok()))
	// 	.fold(Ok(None), |acc, item| {
	// 		println!("fold {:?} - {:?}", acc, item);
	// 		ready(match (&acc, item) {
	// 			// keep value | first error
	// 			(Ok(Some(_)), _) | (Err(_), Err(_)) => acc,
	// 			// use first value
	// 			(Ok(None), Ok(value)) | (Err(_), Ok(value)) => Ok(Some(value)),
	// 			// use error when no value
	// 			(Ok(None), Err(err)) => Err(err),
	// 		})
	// 	})
	// 	.await
}

#[cfg(test)]
mod tests {
	use crate::state::find;
	use cid::Cid;
	use co_primitives::{DagCollection, DefaultNodeSerializer, Node, NodeBuilder, OptionLink};
	use co_storage::{BlockStorage, MemoryBlockStorage};

	#[tokio::test]
	async fn smoke() {
		// test data
		let storage = MemoryBlockStorage::default();
		let mut builder = NodeBuilder::new(storage.max_block_size(), 2, DefaultNodeSerializer::new());
		builder.push(1).unwrap();
		builder.push(2).unwrap();
		builder.push(3).unwrap();
		let (cid, blocks) = builder.into_blocks().unwrap();
		for block in blocks {
			storage.set(block).await.unwrap();
		}
		#[derive(Debug, Default)]
		struct DagVec {
			cid: Option<Cid>,
		}
		impl DagCollection for DagVec {
			type Item = i32;
			type Collection = Vec<i32>;

			fn link(&self) -> OptionLink<Node<Self::Item>> {
				self.cid.into()
			}
			fn set_link(&mut self, link: OptionLink<Node<Self::Item>>) {
				self.cid = link.into();
			}
		}

		// find
		assert_eq!(Some(2), find(&storage, &DagVec { cid: cid.into() }, |i| *i == 2).await.unwrap());
	}
}
