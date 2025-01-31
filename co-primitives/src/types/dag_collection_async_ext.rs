use crate::{BlockStorage, DagCollection, Node, NodeBuilder, NodeStream, OptionLink, StorageError};
use futures::{Stream, TryStreamExt};

#[allow(async_fn_in_trait)]
pub trait DagCollectionAsyncExt: DagCollection {
	fn stream<S: BlockStorage + Clone + Send + Sync + 'static>(
		&self,
		storage: &S,
	) -> impl Stream<Item = Result<Self::Item, StorageError>> + '_
	where
		Self::Item: Send + Sync + 'static,
	{
		NodeStream::from_link(storage.clone(), self.link())
	}

	async fn to_link<S: BlockStorage + Clone + Send + Sync + 'static>(
		storage: &S,
		items: impl IntoIterator<Item = Self::Item>,
	) -> Result<OptionLink<Node<Self::Item>>, StorageError> {
		let mut node_builder = NodeBuilder::<Self::Item, S::StoreParams>::default();
		for item in items {
			node_builder.push(item).unwrap();
			for block in node_builder.take_blocks() {
				storage.set(block).await?;
			}
		}
		let (root, blocks) = node_builder.into_blocks().unwrap();
		for block in blocks {
			storage.set(block).await?;
		}
		Ok(root.into())
	}

	async fn from_link<S: BlockStorage + Clone + Send + Sync + 'static>(
		&self,
		storage: &S,
	) -> Result<Self::Collection, StorageError>
	where
		Self::Item: Send + Sync + 'static,
	{
		Ok(self.stream(storage).try_collect().await?)
	}
}
impl<T> DagCollectionAsyncExt for T where T: DagCollection {}

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
