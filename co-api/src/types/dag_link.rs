use crate::{library::node_reader::node_reader, Storage};
use co_primitives::{DefaultNodeSerializer, NodeBuilder};
use libipld::Cid;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	iter,
	marker::PhantomData,
};

/**
 * Simple trait for creating a DagLink type object
 */
pub trait CreateLink {
	type IntoIterator;
	fn to_link(i: Self::IntoIterator, s: &mut dyn Storage) -> Self;
}

/**
 * Simple trait for recreating pure data from DagLink type object
 */
pub trait FromLink {
	type IntoIterator;
	fn from_link(&self, s: &dyn Storage) -> Self::IntoIterator;
}

pub trait LinkIterator {
	type Item;
	fn iter(&self, s: &dyn Storage) -> impl Iterator<Item = Self::Item>;
}

/**
 * A wrapper type for DagLink types that use vectors
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagVec<V>
where
	V: Clone + Serialize,
{
	pub link: DagLink<Vec<V>, V>,
}

impl<V> CreateLink for DagVec<V>
where
	V: Clone + Serialize,
{
	type IntoIterator = Vec<V>;
	fn to_link(i: Self::IntoIterator, s: &mut dyn Storage) -> Self {
		Self { link: DagLink::to_link(i, s) }
	}
}

impl<V> FromLink for DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	type IntoIterator = Vec<V>;
	fn from_link(&self, s: &dyn Storage) -> Self::IntoIterator {
		self.link.from_link(s)
	}
}

/**
 * A wrapper for DagLink types that use the BTreeSet type
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagSet<V>
where
	V: Ord + Clone + Serialize,
{
	pub link: DagLink<BTreeSet<V>, V>,
}

impl<V> CreateLink for DagSet<V>
where
	V: Ord + Clone + Serialize,
{
	type IntoIterator = BTreeSet<V>;
	fn to_link(i: Self::IntoIterator, s: &mut dyn Storage) -> Self {
		Self { link: DagLink::to_link(i, s) }
	}
}

impl<V> FromLink for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	type IntoIterator = BTreeSet<V>;
	fn from_link(&self, s: &dyn Storage) -> Self::IntoIterator {
		self.link.from_link(s)
	}
}

/**
 * A wrapper for DagLink types that use the BTreeMap type
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagMap<K, V>
where
	K: std::cmp::Ord + Clone + Serialize,
	V: Clone + Serialize,
{
	pub link: DagLink<BTreeMap<K, V>, (K, V)>,
}

impl<K, V> CreateLink for DagMap<K, V>
where
	K: std::cmp::Ord + Clone + Serialize,
	V: Clone + Serialize,
{
	type IntoIterator = BTreeMap<K, V>;
	fn to_link(i: Self::IntoIterator, s: &mut dyn Storage) -> Self {
		Self { link: DagLink::to_link(i, s) }
	}
}

impl<K, V> FromLink for DagMap<K, V>
where
	K: std::cmp::Ord + Clone + Serialize,
	V: Clone + Serialize,
	(K, V): DeserializeOwned + 'static,
{
	type IntoIterator = BTreeMap<K, V>;
	fn from_link(&self, s: &dyn Storage) -> Self::IntoIterator {
		self.link.from_link(s)
	}
}

/**
 * A wrapper type for any iterable data. Will implement FromBlocks and IntoBlocks traits for easy conversion between
 * data and CIDs
 * Types this is mainly used for: Vec, BTreeSet, BTreeMap
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagLink<I, V>
where
	I: IntoIterator<Item = V>,
{
	_p_data: PhantomData<I>,
	pub cid: Option<Cid>,
}

impl<I, V> CreateLink for DagLink<I, V>
where
	I: IntoIterator<Item = V>,
	V: Clone + Serialize,
{
	type IntoIterator = I;
	fn to_link(items: Self::IntoIterator, s: &mut dyn Storage) -> Self {
		let mut node_builder = NodeBuilder::<V>::new(10, DefaultNodeSerializer::new());
		for item in items {
			node_builder.push(item).unwrap();
		}
		let blocks = node_builder.into_blocks().unwrap();
		if let Some(first_block) = blocks.first().cloned() {
			for block in blocks {
				s.set(block);
			}
			Self { _p_data: PhantomData::default(), cid: Some(first_block.cid().clone()) }
		} else {
			Self { _p_data: PhantomData::default(), cid: None }
		}
	}
}

impl<I, V> FromLink for DagLink<I, V>
where
	I: IntoIterator<Item = V> + FromIterator<V>,
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	type IntoIterator = I;
	fn from_link(&self, s: &dyn Storage) -> Self::IntoIterator {
		if let Some(cid) = self.cid {
			let node_reader = node_reader::<V>(s, &cid);
			if let Ok(c) = node_reader.collect() {
				return I::from_iter::<I>(c);
			}
		}
		I::from_iter(iter::empty())
	}
}

// impl<I, V> LinkIterator for DagLink<I, V>
// where
// 	I: IntoIterator<Item = V>,
// 	V: Clone + Serialize + DeserializeOwned + 'static,
// {
// 	type Item = V;
// 	fn iter(&self, s: &dyn Storage) -> impl Iterator<Item = Self::Item> {
// 		if let Some(cid) = self.cid {
// 			let iterator = node_reader::<V>(s, &cid).filter_map(|i| i.ok());
// 			return iterator;
// 		}
// 		iter::empty()
// 	}
// }

#[cfg(test)]
mod test {
	use super::DagSet;
	use crate::{
		types::dag_link::{CreateLink, DagVec, FromLink},
		DagMap,
	};
	use co_storage::{MemoryStorage, Storage};
	use std::collections::{BTreeMap, BTreeSet};

	struct TestStorage {
		mem_storage: MemoryStorage,
	}
	impl crate::Storage for TestStorage {
		fn get(&self, cid: &libipld::Cid) -> crate::Block {
			self.mem_storage.get(cid).expect("get")
		}

		fn set(&mut self, block: crate::Block) -> libipld::Cid {
			self.mem_storage.set(block).expect("set")
		}
	}
	#[test]
	fn test_vec() {
		let mut s = TestStorage { mem_storage: MemoryStorage::new() };
		let original_vec = vec![
			"test".to_owned(),
			"testy".to_owned(),
			"zesty".to_owned(),
			"some".to_owned(),
			"more".to_owned(),
			"strings".to_owned(),
			"to".to_owned(),
			"test".to_owned(),
			"memory".to_owned(),
			"usage".to_owned(),
		];
		let dag_vec = DagVec::to_link(original_vec.clone(), &mut s);
		let restored_vec = dag_vec.from_link(&s);
		println!("Original vector: {:?}", original_vec);
		println!(
			"Sizes:\n\tPure data: {:?}\n\tLink): {:?}",
			std::mem::size_of_val(&*original_vec), // should grow with vector size
			std::mem::size_of_val(&dag_vec)        // should stay at 96 bit
		);
		assert_eq!(original_vec, restored_vec)
	}

	#[test]
	fn test_set() {
		let mut original_set: BTreeSet<String> = BTreeSet::new();

		original_set.insert("test".into());
		original_set.insert("testy".into());
		original_set.insert("test".into());
		original_set.insert("zesty".into());

		let mut s = TestStorage { mem_storage: MemoryStorage::new() };
		let dag_set = DagSet::to_link(original_set.clone(), &mut s);
		let restored_set = dag_set.from_link(&s);
		assert_eq!(original_set, restored_set);
	}

	#[test]
	fn test_map() {
		let mut original_map: BTreeMap<String, String> = BTreeMap::new();

		original_map.insert("test".into(), "test".into());
		original_map.insert("testy".into(), "testy".into());
		original_map.insert("test".into(), "test".into());
		original_map.insert("zesty".into(), "zesty".into());

		let mut s = TestStorage { mem_storage: MemoryStorage::new() };
		let dag_map = DagMap::to_link(original_map.clone(), &mut s);
		let restored_map = dag_map.from_link(&s);
		assert_eq!(original_map, restored_map);
	}
}
