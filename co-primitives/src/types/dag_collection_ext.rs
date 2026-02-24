// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	node_reader, DagCollection, DagMap, DagSet, DagVec, Node, NodeBuilder, NodeReaderError, OptionLink, Storage,
};
use serde::{de::DeserializeOwned, Serialize};

pub trait DagCollectionExt: DagCollection {
	/// Replace contents with collection.
	fn set_collection(&mut self, storage: &mut dyn Storage, items: Self::Collection) {
		self.set_link(Self::write(storage, items))
	}

	/// Materialize into the collection.
	fn collection(&self, storage: &dyn Storage) -> Self::Collection {
		self.read(storage).expect("Valid serialized data")
	}

	fn create(storage: &mut dyn Storage, items: impl IntoIterator<Item = Self::Item>) -> Self {
		let mut result = Self::default();
		result.set_link(Self::write(storage, items));
		result
	}

	fn update<F: FnOnce(&mut dyn Storage, &mut Self::Collection) -> R, R>(
		&mut self,
		storage: &mut dyn Storage,
		f: F,
	) -> R {
		let mut collection = self.collection(storage);
		let result = f(storage, &mut collection);
		self.set_collection(storage, collection);
		result
	}

	fn try_update<F: FnOnce(&mut dyn Storage, &mut Self::Collection) -> Result<R, anyhow::Error>, R>(
		&mut self,
		storage: &mut dyn Storage,
		f: F,
	) -> Result<R, anyhow::Error> {
		let mut collection = self.collection(storage);
		let result = f(storage, &mut collection)?;
		self.set_collection(storage, collection);
		Ok(result)
	}

	fn update_owned<F: FnOnce(&mut dyn Storage, Self::Collection) -> Self::Collection>(
		&mut self,
		storage: &mut dyn Storage,
		f: F,
	) {
		let mut collection = self.collection(storage);
		collection = f(storage, collection);
		self.set_collection(storage, collection);
	}

	fn iter(&self, storage: &dyn Storage) -> impl Iterator<Item = Self::Item> {
		node_reader::<Self::Item>(storage, *self.link().cid()).map(|item| item.expect("Valid serialized data"))
	}

	fn try_iter(&self, storage: &dyn Storage) -> impl Iterator<Item = Result<Self::Item, NodeReaderError>> {
		node_reader::<Self::Item>(storage, *self.link().cid())
	}

	fn write(storage: &mut dyn Storage, items: impl IntoIterator<Item = Self::Item>) -> OptionLink<Node<Self::Item>> {
		let mut node_builder = NodeBuilder::<Self::Item>::default();
		for item in items {
			node_builder.push(item).unwrap();
		}
		let (root, blocks) = node_builder.into_blocks().unwrap();
		for block in blocks {
			storage.set(block);
		}
		root
	}

	fn read(&self, storage: &dyn Storage) -> Result<Self::Collection, NodeReaderError> {
		self.try_iter(storage).collect()
	}
}
impl<T> DagCollectionExt for T where T: DagCollection {}

pub trait DagVecExt: DagCollectionExt {
	/// Update one element that matches the predicate.
	/// Returns Some result if a item has been updated and None otherwise.
	///
	/// TODO: Do not load whole collection into memory.
	fn update_one<R>(
		&mut self,
		storage: &mut dyn Storage,
		predicate: impl Fn(&mut dyn Storage, &Self::Item) -> bool,
		update: impl FnOnce(&mut dyn Storage, &mut Self::Item) -> R,
	) -> Option<R>;
}
impl<V> DagVecExt for DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	fn update_one<R>(
		&mut self,
		storage: &mut dyn Storage,
		predicate: impl Fn(&mut dyn Storage, &Self::Item) -> bool,
		update: impl FnOnce(&mut dyn Storage, &mut Self::Item) -> R,
	) -> Option<R> {
		let mut collection = self.collection(storage);
		for item in collection.iter_mut() {
			if predicate(storage, item) {
				let result = update(storage, item);
				self.set_collection(storage, collection);
				return Some(result);
			}
		}
		None
	}
}

pub trait DagSetExt: DagCollectionExt {
	/// Returns `true` if the set contains no elements.
	fn is_empty(&self) -> bool;

	/// Adds a value to the set.
	///
	/// TODO: (perf): Do not load whole set into memory
	fn insert(&mut self, storage: &mut dyn Storage, value: Self::Item) -> bool;

	/// Remove a value from the set.
	///
	/// TODO: (perf): Do not load whole set into memory
	fn remove(&mut self, storage: &mut dyn Storage, value: &Self::Item) -> bool;

	/// Update one element that matches the predicate.
	/// Returns Some result if a item has been updated and None otherwise.
	///
	/// TODO: Do not load whole collection into memory.
	fn update_one<R>(
		&mut self,
		storage: &mut dyn Storage,
		predicate: impl Fn(&mut dyn Storage, &Self::Item) -> bool,
		update: impl FnOnce(&mut dyn Storage, &mut Self::Item) -> R,
	) -> Option<R>;

	/// Update one element that matches the predicate.
	/// Returns Some result if a item has been updated and None otherwise.
	/// If the update fails it will be not applied.
	///
	/// TODO: Do not load whole collection into memory.
	fn try_update_one<R>(
		&mut self,
		storage: &mut dyn Storage,
		predicate: impl Fn(&mut dyn Storage, &Self::Item) -> bool,
		update: impl FnOnce(&mut dyn Storage, &mut Self::Item) -> Result<R, anyhow::Error>,
	) -> Result<Option<R>, anyhow::Error>;
}
impl<V> DagSetExt for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	/// Returns `true` if the set contains no elements.
	fn is_empty(&self) -> bool {
		self.link().is_none()
	}

	/// Adds a value to the set.
	///
	/// TODO: (perf): Do not load whole set into memory
	fn insert(&mut self, storage: &mut dyn Storage, value: Self::Item) -> bool {
		let mut set = self.collection(storage);
		if set.insert(value) {
			self.set_collection(storage, set);
			true
		} else {
			false
		}
	}

	/// Remove a value from the set.
	///
	/// TODO: (perf): Do not load whole set into memory
	fn remove(&mut self, storage: &mut dyn Storage, value: &Self::Item) -> bool {
		let mut set = self.collection(storage);
		if set.remove(value) {
			self.set_collection(storage, set);
			true
		} else {
			false
		}
	}

	/// Update one element that matches the predicate.
	/// Returns Some result if a item has been updated and None otherwise.
	///
	/// TODO: Do not load whole collection into memory.
	fn update_one<R>(
		&mut self,
		storage: &mut dyn Storage,
		predicate: impl Fn(&mut dyn Storage, &Self::Item) -> bool,
		update: impl FnOnce(&mut dyn Storage, &mut Self::Item) -> R,
	) -> Option<R> {
		let mut collection = self.collection(storage);
		if let Some(mut item) = collection.iter().find(|item| predicate(storage, item)).cloned() {
			if collection.remove(&item) {
				// update
				let result = update(storage, &mut item);

				// insert
				collection.insert(item);
				self.set_collection(storage, collection);
				return Some(result);
			}
		}
		None
	}

	/// Update one element that matches the predicate.
	/// Returns Some result if a item has been updated and None otherwise.
	/// If the update fails it will be not applied.
	///
	/// TODO: Do not load whole collection into memory.
	fn try_update_one<R>(
		&mut self,
		storage: &mut dyn Storage,
		predicate: impl Fn(&mut dyn Storage, &Self::Item) -> bool,
		update: impl FnOnce(&mut dyn Storage, &mut Self::Item) -> Result<R, anyhow::Error>,
	) -> Result<Option<R>, anyhow::Error> {
		let mut collection = self.collection(storage);
		if let Some(mut item) = collection.iter().find(|item| predicate(storage, item)).cloned() {
			if collection.remove(&item) {
				// update
				let result = update(storage, &mut item)?;

				// insert
				collection.insert(item);
				self.set_collection(storage, collection);
				return Ok(Some(result));
			}
		}
		Ok(None)
	}
}

pub trait DagMapExt<K, V>: DagCollectionExt<Item = (K, V)> {
	/// Returns a value corresponding to the key.
	fn get(&mut self, storage: &mut dyn Storage, key: &K) -> Option<V>;

	/// Inserts a key-value pair into the map.
	fn insert(&mut self, storage: &mut dyn Storage, key: K, value: V) -> Option<V>;

	/// Removes a key from the map, returning the value at the key if the key
	/// was previously in the map.
	fn remove(&mut self, storage: &mut dyn Storage, key: &K) -> Option<V>;

	/// Update element with given key.
	/// Returns Some result if a item has been updated and None otherwise.
	fn update_key<R>(
		&mut self,
		storage: &mut dyn Storage,
		key: &K,
		update: impl FnOnce(&mut dyn Storage, &K, &mut V) -> R,
	) -> Option<R>;

	/// Update element with given key.
	/// Returns Some result if the key was found and modified None otherwise.
	fn try_update_key<R>(
		&mut self,
		storage: &mut dyn Storage,
		key: &K,
		update: impl FnOnce(&mut dyn Storage, &K, &mut V) -> Result<R, anyhow::Error>,
	) -> Result<Option<R>, anyhow::Error>;
}
impl<K, V> DagMapExt<K, V> for DagMap<K, V>
where
	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	/// Returns a value corresponding to the key.
	fn get(&mut self, storage: &mut dyn Storage, key: &K) -> Option<V> {
		self.iter(storage)
			.find(|(item_key, _item_value)| key == item_key)
			.map(|(_item_key, item_value)| item_value)
	}

	/// Inserts a key-value pair into the map.
	///
	/// TODO: Do not load whole collection into memory.
	fn insert(&mut self, storage: &mut dyn Storage, key: K, value: V) -> Option<V> {
		self.update(storage, |_, v| v.insert(key, value))
	}

	/// Removes a key from the map, returning the value at the key if the key
	/// was previously in the map.
	///
	/// TODO: Do not load whole collection into memory.
	fn remove(&mut self, storage: &mut dyn Storage, key: &K) -> Option<V> {
		self.update(storage, |_, v| v.remove(key))
	}

	/// Update element with given key.
	/// Returns Some result if a item has been updated and None otherwise.
	fn update_key<R>(
		&mut self,
		storage: &mut dyn Storage,
		key: &K,
		update: impl FnOnce(&mut dyn Storage, &K, &mut V) -> R,
	) -> Option<R> {
		self.update(storage, move |storage, map| {
			if let Some(mut item) = map.remove(key) {
				let result = update(storage, key, &mut item);
				map.insert(key.clone(), item);
				return Some(result);
			}
			None
		})
	}

	/// Update element with given key.
	/// Returns Some result if the key was found and modified None otherwise.
	fn try_update_key<R>(
		&mut self,
		storage: &mut dyn Storage,
		key: &K,
		update: impl FnOnce(&mut dyn Storage, &K, &mut V) -> Result<R, anyhow::Error>,
	) -> Result<Option<R>, anyhow::Error> {
		self.try_update(storage, move |storage, map| {
			if let Some(mut item) = map.remove(key) {
				let result = update(storage, key, &mut item)?;
				map.insert(key.clone(), item);
				return Ok(Some(result));
			}
			Ok(None)
		})
	}
}
