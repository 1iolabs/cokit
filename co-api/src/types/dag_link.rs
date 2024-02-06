use crate::{
	library::{wasm_node_reader::wasm_node_reader, wasm_storage::WasmStorage},
	Block,
};
use co_primitives::{DefaultNodeSerializer, NodeBuilder};
use libipld::Cid;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	marker::PhantomData,
};

pub trait IntoBlocks {
	fn into_blocks(&self) -> Vec<Block>;
}

pub trait FromBlocks {
	fn from_blocks(cid: &Cid, s: &WasmStorage) -> Self;
}

#[derive(PartialEq, Debug)]
pub struct DagVec<V: Clone + Serialize> {
	pub content: DagLink<V, Vec<V>>,
}

impl<V: Clone + Serialize> DagVec<V> {
	pub fn new(content: Vec<V>) -> Self {
		Self { content: DagLink::new(content) }
	}
}

struct DagMap<K: std::cmp::Ord + Clone + Serialize, V: Clone + Serialize> {
	pub content: DagLink<(K, V), BTreeMap<K, V>>,
}

struct DagSet<V: std::cmp::Ord + Clone + Serialize> {
	pub content: DagLink<V, BTreeSet<V>>,
}

#[derive(PartialEq, Debug)]
pub struct DagLink<F: Clone + Serialize, C: IntoIterator + FromIterator<F> + Clone + Serialize> {
	_p_data: PhantomData<F>,
	pub content: C,
}

impl<F: Clone + Serialize, C: IntoIterator + FromIterator<F> + Clone + Serialize> DagLink<F, C> {
	pub fn new(content: C) -> Self {
		Self { _p_data: PhantomData::default(), content }
	}
}

impl<F: Clone + Serialize, C: IntoIterator<Item = F> + FromIterator<F> + Clone + Serialize> IntoBlocks
	for DagLink<F, C>
{
	fn into_blocks(&self) -> Vec<Block> {
		let mut node_builder = NodeBuilder::<F>::new(10, DefaultNodeSerializer::new());
		for item in self.content.clone().into_iter() {
			node_builder.push(item).unwrap();
		}
		node_builder.into_blocks().unwrap()
	}
}

impl<F, C> FromBlocks for DagLink<F, C>
where
	F: Clone + Serialize + DeserializeOwned + 'static,
	C: IntoIterator<Item = F> + FromIterator<F> + Clone + Serialize + Default,
{
	fn from_blocks(cid: &Cid, s: &WasmStorage) -> Self {
		let node_reader = wasm_node_reader::<F>(s, cid);
		if let Ok(c) = node_reader.collect() {
			let content = C::from_iter::<C>(c);
			return Self { _p_data: PhantomData::default(), content }
		}

		Self { _p_data: PhantomData::default(), content: C::default() }
	}
}

mod test {
	use crate::{
		library::{wasm_node_reader::wasm_node_reader, wasm_storage::WasmStorage},
		types::dag_link::{DagLink, DagVec, FromBlocks, IntoBlocks},
		Storage,
	};
	use libipld::{Block, DefaultParams};

	#[test]
	fn test() {
		let vec = DagVec::new(vec!["test".to_owned(), "testy".to_owned(), "zesty".to_owned()]);
		let blocks = vec.content.into_blocks();
		println!("{:?}", blocks);
		let mut s = WasmStorage::new();
		if let Some(first_block) = blocks.first().cloned() {
			for block in blocks {
				s.set(block);
			}
			let restored_vec = DagLink::<String, Vec<String>>::from_blocks(&first_block.cid(), &s);
			assert_eq!(vec.content.content, restored_vec.content)
		}
	}
}
