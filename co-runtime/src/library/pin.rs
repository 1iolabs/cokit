use anyhow::anyhow;
use co_api::Metadata;
use co_storage::{node_reader, DefaultNodeSerializer, NodeBuilder, Storage, StorageError};
use libipld::{cbor::DagCborCodec, prelude::Codec, serde::from_ipld, Cid, Ipld};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug)]
pub struct PinMapping {
	pub pin: Cid,
	pub removed: BTreeSet<Cid>,
	pub added: BTreeSet<Cid>,
}

impl PinMapping {
	/// Create pin mapping from state.
	///
	/// # Arguments
	/// - `state` - The CID of the state we want to pin.
	/// - `pin` - The CID of the pin mapping of a previous state.
	pub fn from_state<S: Storage + Sized>(storage: &mut S, state: Cid, pin: Option<Cid>) -> anyhow::Result<PinMapping> {
		// get current pins
		let mut pins = if let Some(cid) = pin { PinEntry::read(storage, &cid)? } else { Default::default() };

		// compute next pins
		let mut added = BTreeSet::new();
		let mut next_pins = BTreeMap::<Cid, BTreeSet<Cid>>::new();
		compute_entry(&mut pins, &mut next_pins, &mut added, storage, &state)?;

		// all items left in current pins are not used anymore
		let removed = pins
			.into_iter()
			.map(|(k, _)| k)
			// .flat_map(|(k, v)| [k].into_iter().chain(v.into_iter()))
			.collect::<BTreeSet<Cid>>();

		// store next pins
		let next_pin = PinEntry::write(storage, &next_pins)?;

		// result
		Ok(PinMapping { pin: next_pin, removed, added })
	}
}

fn compute_entry<S: Storage>(
	from: &mut BTreeMap<Cid, BTreeSet<Cid>>,
	to: &mut BTreeMap<Cid, BTreeSet<Cid>>,
	added: &mut BTreeSet<Cid>,
	storage: &S,
	cid: &Cid,
) -> anyhow::Result<()> {
	if !move_entry(from, to, &cid) {
		// find children
		let ipld = decode(storage, &cid)?;
		let mut children = Default::default();
		find_ipld_cids(&mut children, storage, &ipld);

		// children
		for child in children.iter() {
			compute_entry(from, to, added, storage, child)?;
		}

		// add
		added.insert(cid.clone());
		to.insert(cid.clone(), children);
	}
	Ok(())
}

/// Try to move an entry and all its child.
/// Returns true if cid was moved (and therefore already known in from).
fn move_entry(from: &mut BTreeMap<Cid, BTreeSet<Cid>>, to: &mut BTreeMap<Cid, BTreeSet<Cid>>, cid: &Cid) -> bool {
	if let Some((key, value)) = from.remove_entry(cid) {
		for child_cid in value.iter() {
			move_entry(from, to, child_cid);
		}
		to.insert(key, value);
		true
	} else {
		false
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
	fn read<S: Storage>(storage: &S, cid: &Cid) -> anyhow::Result<BTreeMap<Cid, BTreeSet<Cid>>> {
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

	fn write<S: Storage>(storage: &mut S, map: &BTreeMap<Cid, BTreeSet<Cid>>) -> anyhow::Result<Cid> {
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

#[derive(Debug, Default, Serialize, Deserialize)]
struct PinState {
	pins: BTreeMap<Cid, BTreeSet<Cid>>,
}

#[derive(Debug, thiserror::Error)]
enum DecodeError {
	#[error("Storage Error")]
	Storage(#[from] StorageError),
	#[error("Decode Error")]
	Decode(#[from] anyhow::Error),
	#[error("Unsupported Codec Error")]
	UnsupportedCodec,
}

fn decode<S: Storage>(storage: &S, cid: &Cid) -> Result<Ipld, DecodeError> {
	if cid.codec() == Into::<u64>::into(DagCborCodec) {
		let block = storage.get(cid)?;
		let ipld: Ipld = DagCborCodec.decode(block.data())?;
		Ok(ipld)
	} else {
		Err(DecodeError::UnsupportedCodec)
	}
}

/// Recursively find all referenced `Cid`'s in an `Ipld` data structure by calling `found`.
fn find_ipld_cids<S: Storage>(result: &mut BTreeSet<Cid>, storage: &S, ipld: &Ipld) {
	match ipld {
		Ipld::List(v) =>
			for i in v.iter() {
				find_ipld_cids(result, storage, i);
			},
		Ipld::Map(v) => {
			let external = get_external(ipld).unwrap_or_default();
			for (k, i) in v.iter() {
				if !external.contains(k) {
					find_ipld_cids(result, storage, i);
				}
			}
		},
		Ipld::Link(v) => {
			result.insert(v.clone());
		},
		_ => {},
	}
}

// pub fn find_external(ipld: &Ipld) -> Vec<String> {
//     let mut result = Vec::new();

//     let metadata: Vec<Metadata> = from_ipld(ipld.get("$co"));

//     match ipld {
//         Ipld::Map(v) => match v.get("$co") {
//             Some(Ipld::List(r)) => {
//                 for k in r.iter() {
//                     // {"ext": {"f": ["..."]}}
//                     match k {
//                         Ipld::Map(metadata) => {
//                             match metadata.first_key_value() {
//                                 Some((k, v)) if k == "ext" => {},
//                                 _ => {},
//                             }
//                         },
//                         _ => {},
//                     }
//                 }
//             },
//             _ => {},
//         }
//         _ => {},
//     }
//     result
// }

enum GetExternalError {
	NotFound,
	Decode,
}

fn get_external(ipld: &Ipld) -> Result<HashSet<String>, GetExternalError> {
	Ok(from_ipld::<Vec<Metadata>>(ipld.get("$co").map_err(|_| GetExternalError::NotFound)?.clone())
		.map_err(|_| GetExternalError::Decode)?
		.into_iter()
		.filter_map(|v| match v {
			Metadata::External(f) => Some(f),
		})
		.flatten()
		.map(|s| s.to_owned())
		.collect())
}

// let ext = ipld_field_list(ipld, "$co")
//         .iter()
//         .filter_map(|v| ipld_field(v, "ext"))
//         .map(|ext| ipld_field_list(ext, "f"))
//         .flatten()
//         .filter_map(|f| f)
//         .collect();

// fn ipld_field_string<'a>(ipld: &'a Ipld, name: &str) -> Option<&'a String> {
//     match ipld_field(ipld, name) {
//         Some(Ipld::String(r)) => Some(r),
//         _ => None,
//     }
// }

// fn ipld_field_list<'a>(ipld: &'a Ipld, name: &str) -> &'a Vec<Ipld> {
//     match ipld_field(ipld, name) {
//         Some(Ipld::List(r)) => r,
//         _ => &Vec::new(),
//     }
// }

// fn ipld_field<'a>(ipld: &'a Ipld, name: &str) -> Option<&'a Ipld> {
//     match ipld {
//         Ipld::Map(m) => {
//             m.get(name)
//         },
//         _ => None,
//     }
// }

#[cfg(test)]
mod tests {
	use super::PinMapping;
	use co_storage::{BlockSerializer, MemoryStorage, Storage};
	use libipld::Cid;
	use serde::{Deserialize, Serialize};

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
	fn get_reference_storage() -> (MemoryStorage, Vec<Cid>) {
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

		let mut storage = MemoryStorage::new();
		storage.set(cid6.clone()).unwrap();
		storage.set(cid5.clone()).unwrap();
		storage.set(cid4.clone()).unwrap();
		storage.set(cid3.clone()).unwrap();
		storage.set(cid2.clone()).unwrap();
		storage.set(cid1.clone()).unwrap();

		(storage, cids)
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
	fn get_reference_storage_v2(storage: &mut MemoryStorage) -> Vec<Cid> {
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

		storage.set(cid6.clone()).unwrap();
		storage.set(cid5.clone()).unwrap();
		storage.set(cid4.clone()).unwrap();
		storage.set(cid3.clone()).unwrap();
		storage.set(cid2.clone()).unwrap();
		storage.set(cid1.clone()).unwrap();

		cids
	}

	#[test]
	fn new_state() {
		let (mut storage, cids) = get_reference_storage();
		let pin = PinMapping::from_state(&mut storage, cids.get(0).unwrap().clone(), None).unwrap();
		assert_eq!(pin.added.len(), 6);
		for cid in cids.iter() {
			assert!(pin.added.contains(cid));
		}
	}

	#[test]
	fn change_cid5_name() {
		// reference
		let (mut storage, cids) = get_reference_storage();
		let pin = PinMapping::from_state(&mut storage, cids.get(0).unwrap().clone(), None).unwrap();

		let next_cids = get_reference_storage_v2(&mut storage);
		let next_pin = PinMapping::from_state(&mut storage, next_cids.get(0).unwrap().clone(), Some(pin.pin)).unwrap();
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
