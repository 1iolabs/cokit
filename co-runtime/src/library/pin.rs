use anyhow::anyhow;
use cid::Cid;
use co_primitives::{DefaultNodeSerializer, NodeBuilder};
use co_storage::{node_reader, Storage, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PinEntry {
	#[serde(rename = "r")]
	Root(Cid),
	#[serde(rename = "c")]
	Child(Cid),
}
impl PinEntry {
	fn _read<S: Storage>(storage: &S, cid: &Cid) -> anyhow::Result<BTreeMap<Cid, BTreeSet<Cid>>> {
		let mut result = BTreeMap::<Cid, BTreeSet<Cid>>::new();
		let mut root = None;
		for item in node_reader(storage, cid) {
			match item? {
				PinEntry::Root(cid) => {
					root = Some(cid);
					result.entry(cid).or_default();
				},
				PinEntry::Child(cid) => {
					// ensure root
					result.entry(cid).or_default();

					// child
					result
						.get_mut(&root.ok_or(StorageError::InvalidArgument(anyhow!("No root id")))?)
						.ok_or(StorageError::InvalidArgument(anyhow!("No root")))?
						.insert(cid);
				},
			}
		}
		Ok(result)
	}

	fn _write<S: Storage>(storage: &mut S, map: &BTreeMap<Cid, BTreeSet<Cid>>) -> anyhow::Result<Cid> {
		// validate
		if map.is_empty() {
			return Err(StorageError::InvalidArgument(anyhow!("Empty")))?;
		}

		// build
		let mut builder = NodeBuilder::<PinEntry, DefaultNodeSerializer, S::StoreParams>::default();
		for (k, v) in map.iter() {
			// skip empty sets as their CID's will be added as childs by referencing roots
			// in case of single root state onyl write the root node
			if !v.is_empty() || map.len() == 1 {
				builder.push(PinEntry::Root(*k))?;
				for c in v.iter() {
					builder.push(PinEntry::Child(*c))?;
				}
			}
		}

		// store
		let blocks = builder.into_blocks()?;
		let result = *blocks.first().expect("at leat one block").cid();
		for block in blocks.into_iter() {
			storage.set(block)?;
		}

		// result
		Ok(result)
	}
}
