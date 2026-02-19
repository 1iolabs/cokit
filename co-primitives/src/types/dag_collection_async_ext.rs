// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	types::streamable::Streamable, BlockStorage, DagCollection, Node, NodeBuilder, NodeStream, OptionLink, StorageError,
};
use futures::TryStreamExt;

#[allow(async_fn_in_trait)]
pub trait DagCollectionAsyncExt: DagCollection {
	fn stream<S>(&self, storage: &S) -> NodeStream<S, Self::Item, Node<Self::Item>>
	where
		S: BlockStorage + Clone + Send + Sync + 'static,
		Self::Item: Send + Sync + 'static,
	{
		NodeStream::from_link(storage.to_owned(), self.link())
	}

	async fn write<S: BlockStorage + Clone + Send + Sync + 'static>(
		storage: &S,
		items: impl IntoIterator<Item = Self::Item>,
	) -> Result<OptionLink<Node<Self::Item>>, StorageError> {
		let mut node_builder = NodeBuilder::<Self::Item>::default();
		for item in items {
			node_builder.push(item).map_err(|err| StorageError::Internal(err.into()))?;
			for block in node_builder.take_blocks() {
				storage.set(block).await?;
			}
		}
		let (root, blocks) = node_builder.into_blocks().map_err(|err| StorageError::Internal(err.into()))?;
		for block in blocks {
			storage.set(block).await?;
		}
		Ok(root)
	}

	async fn read<S: BlockStorage + Clone + Send + Sync + 'static>(
		&self,
		storage: &S,
	) -> Result<Self::Collection, StorageError>
	where
		Self::Item: Send + Sync + 'static,
	{
		self.stream(storage).try_collect().await
	}
}
impl<T> DagCollectionAsyncExt for T where T: DagCollection {}
impl<T, S> Streamable<S> for T
where
	T: DagCollectionAsyncExt,
	T::Item: Send + Sync + 'static,
	S: BlockStorage + Clone + 'static,
{
	type Item = Result<T::Item, StorageError>;
	type Stream = NodeStream<S, T::Item, Node<T::Item>>;

	fn stream(&self, storage: S) -> Self::Stream {
		NodeStream::from_link(storage, self.link())
	}
}

// pub trait DagMapAsyncExt<K, V>: DagCollectionExt
// where
// 	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
// 	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
// {
// }
// impl<K, V> DagMapAsyncExt<K, V> for DagMap<K, V>
// where
// 	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
// 	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
// {
// }

// pub struct DagMapTransaction<S, K, V>
// where
// 	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
// 	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
// {
// 	map: DagMap<K, V>,
// 	storage: S,
// 	changes: BTreeMap<K, DagMapChange<V>>,
// }
// impl<S, K, V> DagMapTransaction<S, K, V>
// where
// 	S: BlockStorage,
// 	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
// 	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
// {
// 	pub fn insert(&mut self, key: K, value: V) {
// 		self.changes.insert(key, DagMapChange::Insert(value));
// 	}

// 	pub fn remove(&mut self, key: &K) {
// 		self.changes.insert(key.clone(), DagMapChange::Remove);
// 	}

// 	pub fn get(&self, key: &K) -> Option<V> {
// 		match self.changes.get(key) {
// 			Some(DagMapChange::Insert(value)) => {
// 				return Some(value.clone());
// 			},
// 			Some(DagMapChange::Remove) => {
// 				return None;
// 			},
// 			_ => {
// 				// self.map.stream(self.storage);
// 				todo!();
// 			},
// 		}
// 	}
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// enum DagMapChange<V> {
// 	Insert(V),
// 	Remove,
// }
