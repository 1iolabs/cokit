use crate::{CidResolverBox, MultiLayerCidResolver};
use anyhow::anyhow;
use co_primitives::{DefaultNodeSerializer, NodeBuilder};
use co_storage::{node_reader, Storage, StorageError};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	path::PathBuf,
};
use tokio::fs;

#[derive(Debug)]
pub struct PinMapping {
	pub pin: BTreeMap<Cid, BTreeSet<Cid>>,
	pub removed: BTreeSet<Cid>,
	pub added: BTreeSet<Cid>,
}

impl PinMapping {
	pub async fn from_file(
		application_path: PathBuf,
		resolver: &CidResolverBox,
		state: &Cid,
	) -> anyhow::Result<PinMapping> {
		// read old pin map from application path file
		let content = fs::read(&application_path.with_file_name("pins.cbor")).await?;
		let old_pins: BTreeMap<Cid, BTreeSet<Cid>> = serde_ipld_dagcbor::from_slice(&content)?;

		// create cid resolver
		let mut cid_resolver = MultiLayerCidResolver::new().with_previous_cids(old_pins);
		// compute next pins
		cid_resolver.resolve_cid(state, resolver).await?;
		let (removed, added) = cid_resolver.diff()?;
		Ok(PinMapping { pin: cid_resolver.new_cids()?, removed, added })
	}
}

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
					root = Some(cid.clone());
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
			return Err(StorageError::InvalidArgument(anyhow!("Empty")))?
		}

		// build
		let mut builder = NodeBuilder::<PinEntry, DefaultNodeSerializer, S::StoreParams>::default();
		for (k, v) in map.iter() {
			// skip empty sets as their CID's will be added as childs by referencing roots
			// in case of single root state onyl write the root node
			if !v.is_empty() || map.len() == 1 {
				builder.push(PinEntry::Root(k.clone()))?;
				for c in v.into_iter() {
					builder.push(PinEntry::Child(c.clone()))?;
				}
			}
		}

		// store
		let blocks = builder.into_blocks()?;
		let result = blocks.get(0).expect("at leat one block").cid().clone();
		for block in blocks.into_iter() {
			storage.set(block)?;
		}

		// result
		Ok(result)
	}
}

#[cfg(test)]
mod tests {
	use super::PinMapping;
	use crate::create_cid_resolver;
	use co_primitives::BlockSerializer;
	use co_storage::{BlockStorage, MemoryBlockStorage};
	use libipld::Cid;
	use serde::{Deserialize, Serialize};
	use std::path::PathBuf;

	#[derive(Debug, Serialize, Deserialize)]
	struct TestNode {
		name: String,
		children: Vec<Cid>,
	}

	/// Create test reference model
	/// ┌──────┐
	/// │ CID1 ├────┬────────┐
	/// └──┬───┘    │        │
	///    │        │        │
	/// ┌──▼───┐ ┌──▼───┐ ┌──▼───┐
	/// │ CID2 │ │ CID4 │ │ CID6 │
	/// └──┬───┘ └──┬───┘ └──────┘
	///    │        │
	/// ┌──▼───┐ ┌──▼───┐
	/// │ CID3 │ │ CID5 │
	/// └──────┘ └──────┘
	async fn get_reference_storage() -> anyhow::Result<(MemoryBlockStorage, Vec<Cid>)> {
		let s = BlockSerializer::default();
		let cid6 = s.serialize(&TestNode { name: "CID6".to_owned(), children: vec![] }).unwrap();
		let cid5 = s.serialize(&TestNode { name: "CID5".to_owned(), children: vec![] }).unwrap();
		let cid4 = s
			.serialize(&TestNode { name: "CID4".to_owned(), children: vec![cid5.cid().clone()] })
			.unwrap();
		let cid3 = s.serialize(&TestNode { name: "CID3".to_owned(), children: vec![] }).unwrap();
		let cid2 = s
			.serialize(&TestNode { name: "CID2".to_owned(), children: vec![cid3.cid().clone()] })
			.unwrap();
		let cid1 = s
			.serialize(&TestNode {
				name: "CID1".to_owned(),
				children: vec![cid2.cid().clone(), cid4.cid().clone(), cid6.cid().clone()],
			})
			.unwrap();

		let cids: Vec<Cid> = vec![&cid1, &cid2, &cid3, &cid4, &cid5, &cid6]
			.into_iter()
			.map(|block| block.cid().clone())
			.collect();
		// for (i, cid) in cids.iter().enumerate() {
		// 	println!("cid{}: {:?}", i + 1, cid);
		// }
		// cid1: Cid(bafyr4ifa6hobpbi3eqe36p6u53pd5u3g5kcdwz5vvyou7p4wqgpnrlbtye)
		// cid2: Cid(bafyr4idniluineg7wa2gh5y4kcljvwqj4tzdh2ak42j7xrxkbzblsbjdou)
		// cid3: Cid(bafyr4if2vvjdpqmm3vwbm67rumkbsidwibykyul56iwk56xv3ly6mjhe7u)
		// cid4: Cid(bafyr4iapuv4ln5dzya5l4y2towoztmenyzcm3tlsvjxruicbcgc2n6jlta)
		// cid5: Cid(bafyr4ih7sbwsz7yu6gcilheb5tnaihph45pouftde7yi7cxqcqjqyrwxey)
		// cid6: Cid(bafyr4iezz44ryyhzi53oszexkq42ih6ktoxixkh4tjp42gwltngzb2igk4)

		let storage = MemoryBlockStorage::new();
		storage.set(cid6.clone()).await?;
		storage.set(cid5.clone()).await?;
		storage.set(cid4.clone()).await?;
		storage.set(cid3.clone()).await?;
		storage.set(cid2.clone()).await?;
		storage.set(cid1.clone()).await?;

		Ok((storage, cids))
	}

	/// Create test reference model
	/// ┌──────┐
	/// │ CID1 ├────┬────────┐
	/// └──┬───┘    │        │
	///    │        │        │
	/// ┌──▼───┐ ┌──▼───┐ ┌──▼───┐
	/// │ CID2 │ │ CID4 │ │ CID6 │
	/// └──┬───┘ └──┬───┘ └──────┘
	///    │        │
	/// ┌──▼───┐ ┌──▼──────────┐
	/// │ CID3 │ │ CID5_RENAME │
	/// └──────┘ └─────────────┘
	async fn get_reference_storage_v2(storage: &mut MemoryBlockStorage) -> anyhow::Result<Vec<Cid>> {
		let s = BlockSerializer::default();
		let cid6 = s.serialize(&TestNode { name: "CID6".to_owned(), children: vec![] }).unwrap();
		let cid5 = s
			.serialize(&TestNode { name: "CID5_RENAME".to_owned(), children: vec![] })
			.unwrap();
		let cid4 = s
			.serialize(&TestNode { name: "CID4".to_owned(), children: vec![cid5.cid().clone()] })
			.unwrap();
		let cid3 = s.serialize(&TestNode { name: "CID3".to_owned(), children: vec![] }).unwrap();
		let cid2 = s
			.serialize(&TestNode { name: "CID2".to_owned(), children: vec![cid3.cid().clone()] })
			.unwrap();
		let cid1 = s
			.serialize(&TestNode {
				name: "CID1".to_owned(),
				children: vec![cid2.cid().clone(), cid4.cid().clone(), cid6.cid().clone()],
			})
			.unwrap();

		let cids: Vec<Cid> = vec![&cid1, &cid2, &cid3, &cid4, &cid5, &cid6]
			.into_iter()
			.map(|block| block.cid().clone())
			.collect();
		// for (i, cid) in cids.iter().enumerate() {
		// 	println!("cid{}: {:?}", i + 1, cid);
		// }
		// cid1: Cid(bafyr4ia4yy3a6xjapek5c2e4n2lqp2uoidrp6mityqcpyokax5lpbgf4ve)
		// cid2: Cid(bafyr4idniluineg7wa2gh5y4kcljvwqj4tzdh2ak42j7xrxkbzblsbjdou)
		// cid3: Cid(bafyr4if2vvjdpqmm3vwbm67rumkbsidwibykyul56iwk56xv3ly6mjhe7u)
		// cid4: Cid(bafyr4idglh2kqj7pom2k572ikcv5ptgzz4l4ejwuhietdxpdwpbj2z5cxm)
		// cid5: Cid(bafyr4icgsnke4yqo53ve65mw7ctvexn42jsw7xg23ewdfsfdv6lq6eq73e)
		// cid6: Cid(bafyr4iezz44ryyhzi53oszexkq42ih6ktoxixkh4tjp42gwltngzb2igk4)

		storage.set(cid6.clone()).await?;
		storage.set(cid5.clone()).await?;
		storage.set(cid4.clone()).await?;
		storage.set(cid3.clone()).await?;
		storage.set(cid2.clone()).await?;
		storage.set(cid1.clone()).await?;

		Ok(cids)
	}

	#[tokio::test]
	async fn new_state() {
		// TODO create test file
		let (storage, cids) = get_reference_storage().await.expect("storage");
		let resolver = create_cid_resolver(vec![storage]).await.expect("resovlers");
		let pin = PinMapping::from_file(PathBuf::new(), &resolver, &Cid::default())
			.await
			.expect("pin");
		assert_eq!(pin.added.len(), 6);
		for cid in cids.iter() {
			assert!(pin.added.contains(cid));
		}
	}

	#[tokio::test]
	async fn change_cid5_name() {
		// reference
		// TODO create test file
		let (mut storage, cids) = get_reference_storage().await.expect("storage");
		let resolver = create_cid_resolver(vec![storage.clone()]).await.expect("resovlers");
		let _pin = PinMapping::from_file(PathBuf::new(), &resolver, &cids.get(0).unwrap())
			.await
			.expect("pin");

		let next_cids = get_reference_storage_v2(&mut storage).await.expect("next cids");
		let resolver = create_cid_resolver(vec![storage.clone()]).await.expect("resovlers");
		let next_pin = PinMapping::from_file(PathBuf::new(), &resolver, &next_cids.get(0).unwrap())
			.await
			.expect("pin");
		assert_eq!(next_pin.added.len(), 3);
		assert!(next_pin.added.contains(next_cids.get(0).unwrap()));
		assert!(next_pin.added.contains(next_cids.get(3).unwrap()));
		assert!(next_pin.added.contains(next_cids.get(4).unwrap()));
		assert_eq!(next_pin.removed.len(), 3);
		assert!(next_pin.removed.contains(cids.get(0).unwrap()));
		assert!(next_pin.removed.contains(cids.get(3).unwrap()));
		assert!(next_pin.removed.contains(cids.get(4).unwrap()));
	}
}
