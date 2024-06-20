use anyhow::anyhow;
use async_trait::async_trait;
use co_api::{Cid, Metadata};
use co_storage::BlockStorage;
use colored::Colorize;
use libipld::{cbor::DagCborCodec, prelude::Codec, serde::from_ipld, Ipld};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// A shorthand type for a map that represents the structure of a Cid tree
pub type CidMap = BTreeMap<Cid, BTreeSet<Cid>>;

#[async_trait]
pub trait CidResolver {
	async fn resolve(&self, cid: &Cid, ignorable_cids: &BTreeSet<&Cid>) -> Result<BTreeSet<Cid>, anyhow::Error>;
}

pub type CidResolverBox = Box<dyn CidResolver + Send + Sync + 'static>;

/// Resolves Cids by trying to get it from all given BlockStorages
#[derive(Clone)]
pub struct IpldResolver<S>
where
	S: BlockStorage,
{
	pub storages: Vec<S>,
}

impl<S> IpldResolver<S>
where
	S: BlockStorage,
{
	pub fn get_next_cids(ipld: &Ipld, new_cids: &mut BTreeSet<Cid>, ignorable_cids: &BTreeSet<&Cid>) {
		match ipld {
			// found cid -> add to list of new cids
			Ipld::Link(cid) => {
				// no need to resolve cid again if already known
				if !ignorable_cids.contains(cid) {
					new_cids.insert(*cid);
				}
			},
			// found list -> traverse as links might be contained
			Ipld::List(list) => {
				for ipld_inner in list {
					IpldResolver::<S>::get_next_cids(ipld_inner, new_cids, ignorable_cids);
				}
			},
			// found map -> traverse as links might be contained
			Ipld::Map(map) => {
				let external = get_external(ipld).unwrap_or_default();
				for (k, i) in map.iter() {
					// don't resolve encryption mapping cids
					// TODO think of a better way than doing this hard coded
					if !external.contains(k) && k != "encryption_mapping" {
						IpldResolver::<S>::get_next_cids(i, new_cids, ignorable_cids);
					}
				}
			},
			// No need to check further as no other types can contain links
			_ => (),
		};
	}
}

#[async_trait]
impl<S> CidResolver for IpldResolver<S>
where
	S: BlockStorage + Send + Sync,
{
	async fn resolve(&self, cid: &Cid, ignorable_cids: &BTreeSet<&Cid>) -> Result<BTreeSet<Cid>, anyhow::Error> {
		// variable to return last error, initialized value will be returned only if no loop happens
		let mut last_error = Err(anyhow!("no storages given"));
		for storage in self.storages.iter() {
			// try to get block with storage
			let block_result = storage.get(cid).await;
			match block_result {
				Ok(block) => {
					// decode to ipld
					let ipld: Ipld = DagCborCodec.decode(block.data())?;
					let mut links: BTreeSet<Cid> = BTreeSet::new();
					IpldResolver::<S>::get_next_cids(&ipld, &mut links, ignorable_cids);
					// returns first successfully resolved cids
					return Ok(links);
				},
				Err(e) => {
					// save error and try next storage
					last_error = Err(anyhow!(e));
					continue;
				},
			}
		}
		last_error
	}
}

/// Resolver that combines any number of other resolvers.
/// Calls the resolve functions of each of those and returns the first result that is not an Err.
/// Fails if all resolvers fail.
pub struct JoinCidResolver {
	pub resolvers: Vec<CidResolverBox>,
}

impl JoinCidResolver {
	pub fn new(resolvers: Vec<CidResolverBox>) -> Self {
		Self { resolvers }
	}
}

#[async_trait]
impl CidResolver for JoinCidResolver {
	async fn resolve(&self, cid: &Cid, ignorable_cids: &BTreeSet<&Cid>) -> Result<BTreeSet<Cid>, anyhow::Error> {
		for resolver in self.resolvers.iter() {
			if let Ok(result) = resolver.resolve(cid, ignorable_cids).await {
				return Ok(result);
			}
		}
		Err(anyhow!("No given resolver worked"))
	}
}

pub async fn create_cid_resolver<S>(storages: Vec<S>) -> anyhow::Result<CidResolverBox>
where
	S: BlockStorage + Send + Sync + 'static,
{
	Ok(Box::new(JoinCidResolver::new(vec![Box::new(IpldResolver { storages })])))
}

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

/// Contains all relevant data after running the MultiLayerCidResolver.
/// Diff items will be Option::None if no previous cids were given
pub struct MultiLayerCidResolverResult {
	pub new_cid_map: CidMap,
	pub failed_cids: BTreeSet<Cid>,
	pub traversed_layers: i64,
	pub maximum_layers: i64,
	pub added_cids: BTreeSet<Cid>,
	pub removed_cids: BTreeSet<Cid>,
}

impl From<MultiLayerCidResolver> for MultiLayerCidResolverResult {
	fn from(resolver: MultiLayerCidResolver) -> Self {
		MultiLayerCidResolverResult {
			new_cid_map: resolver.new_cids,
			failed_cids: resolver.failed_cids,
			traversed_layers: resolver.current_depth,
			maximum_layers: resolver.depth_limit,
			added_cids: resolver.added_cids,
			removed_cids: match resolver.previous_cids {
				Some(p) => p.keys().cloned().collect(),
				None => BTreeSet::default(),
			},
		}
	}
}

impl MultiLayerCidResolverResult {
	pub fn print_results(&self) {
		// print information of found cid map
		self.print_new_cid_map();

		// diff info
		self.print_diff();

		// depth info
		self.print_layers_info();
	}

	pub fn print_new_cid_map(&self) {
		for (cid, children) in self.new_cid_map.iter() {
			// print found cid
			println!("Cid: {}", cid);

			// print all children
			for child in children {
				let mut child_string = child.to_string().bright_white();
				// mark child if cid could not be resolved
				if self.failed_cids.contains(child) {
					child_string = child_string.red();
				}
				println!("\t{}", child_string);
			}
		}
	}

	pub fn print_layers_info(&self) {
		if self.maximum_layers < 0 {
			println!("Looked in unlimited depth and got to {}", self.traversed_layers);
		} else {
			println!("Looked up to depth {} and got to {}", self.maximum_layers, self.traversed_layers);
		}
	}

	pub fn print_diff(&self) {
		println!("Added items:");
		for i in self.added_cids.iter() {
			println!("{i}");
		}
		println!("Removed items:");
		for i in self.removed_cids.iter() {
			println!("{i}");
		}
	}
}

/// Resolves a cid. Then Looks for other cids and tries to recursively resolve those as well.
/// When given a previous cid map, will simultaneously compute a diff.
/// Consumes itself when calling the resolve function to prevent errors caused by multiple executions.
#[derive(Debug)]
pub struct MultiLayerCidResolver {
	/// Contains all cids that have been found after running
	new_cids: CidMap,
	/// Contains all cids that couldn't be resolved further
	failed_cids: BTreeSet<Cid>,
	/// Optional cid map from a previous state. Will simultaneously run a diff when this is set
	previous_cids: Option<CidMap>,
	/// Defines a depth up to which Links should be resolved. No limit if depth is negtive
	depth_limit: i64,
	/// Tracks depth. After running, shows how many layers of links got resolved.
	current_depth: i64,
	/// After running, contains all cids that were not in the previous map but in the new one
	added_cids: BTreeSet<Cid>,
	/// Internally used
	discovered_cids: BTreeSet<Cid>,
}

impl Default for MultiLayerCidResolver {
	fn default() -> Self {
		Self::new()
	}
}

impl MultiLayerCidResolver {
	pub fn new() -> Self {
		Self {
			new_cids: BTreeMap::new(),
			discovered_cids: BTreeSet::new(),
			failed_cids: BTreeSet::new(),
			added_cids: BTreeSet::new(),
			previous_cids: None,
			depth_limit: -1,
			current_depth: 0,
		}
	}

	/// implements a depth limit for calculations. Limit should never be set for real calculations
	pub fn with_depth_limit(mut self, depth: i64) -> Self {
		self.depth_limit = depth;
		self
	}
	/// additionally calculates which cids are left over from the previous cid map
	pub fn with_previous_cids(mut self, previous_cids: CidMap) -> Self {
		self.previous_cids = Some(previous_cids);
		self
	}

	/// Resolves a cid using the given resolvers. This will consume this resolver as it should not be run multiple
	/// times. Returns a MultiLayerCidResolverResult
	pub async fn resolve_cid(mut self, cid: &Cid, resolver: &CidResolverBox) -> MultiLayerCidResolverResult {
		self.discovered_cids.insert(*cid);
		// resolve cids as long as there are new ones
		while !self.discovered_cids.is_empty() {
			// check if we reached defined depth
			if self.depth_limit >= 0 && self.depth_limit <= self.current_depth {
				break;
			} else {
				self.current_depth += 1;
			}
			// copy new cids for this iteration (to not iterate over a mutable set)
			let new_cids = self.discovered_cids.clone();
			self.discovered_cids.clear();
			// move all new cids to added set
			for new_cid in new_cids.iter() {
				match &mut self.previous_cids {
					Some(p) => {
						if !move_entry(p, &mut self.new_cids, new_cid) {
							// cid is new: try to resolve it and insert into added list
							self.resolve(new_cid, resolver).await;
							self.added_cids.insert(*new_cid);
						}
					},
					None => {
						self.resolve(new_cid, resolver).await;
						self.added_cids.insert(*new_cid);
					},
				}
			}
		}
		self.into()
	}

	async fn resolve(&mut self, cid: &Cid, resolver: &CidResolverBox) {
		if let Ok(mut links) = resolver.resolve(cid, &self.new_cids.keys().collect()).await {
			self.new_cids.insert(*cid, links.clone());
			self.discovered_cids.append(&mut links);
			self.failed_cids.remove(&cid.clone());
		} else {
			self.failed_cids.insert(*cid);
		}
	}
}

/// Try to move an entry and all its children.
/// Returns true if cid was moved (and therefore already known in from).
fn move_entry(from: &mut CidMap, to: &mut CidMap, cid: &Cid) -> bool {
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

#[cfg(test)]
mod tests {
	use crate::{create_cid_resolver, MultiLayerCidResolver};
	use co_primitives::BlockSerializer;
	use co_storage::{BlockStorage, MemoryBlockStorage};
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
	async fn get_reference_storage() -> anyhow::Result<(MemoryBlockStorage, Vec<Cid>)> {
		let s = BlockSerializer::default();
		let cid6 = s.serialize(&TestNode { name: "CID6".to_owned(), children: vec![] }).unwrap();
		let cid5 = s.serialize(&TestNode { name: "CID5".to_owned(), children: vec![] }).unwrap();
		let cid4 = s
			.serialize(&TestNode { name: "CID4".to_owned(), children: vec![*cid5.cid()] })
			.unwrap();
		let cid3 = s.serialize(&TestNode { name: "CID3".to_owned(), children: vec![] }).unwrap();
		let cid2 = s
			.serialize(&TestNode { name: "CID2".to_owned(), children: vec![*cid3.cid()] })
			.unwrap();
		let cid1 = s
			.serialize(&TestNode { name: "CID1".to_owned(), children: vec![*cid2.cid(), *cid4.cid(), *cid6.cid()] })
			.unwrap();

		let cids: Vec<Cid> = vec![&cid1, &cid2, &cid3, &cid4, &cid5, &cid6]
			.into_iter()
			.map(|block| *block.cid())
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
			.serialize(&TestNode { name: "CID4".to_owned(), children: vec![*cid5.cid()] })
			.unwrap();
		let cid3 = s.serialize(&TestNode { name: "CID3".to_owned(), children: vec![] }).unwrap();
		let cid2 = s
			.serialize(&TestNode { name: "CID2".to_owned(), children: vec![*cid3.cid()] })
			.unwrap();
		let cid1 = s
			.serialize(&TestNode { name: "CID1".to_owned(), children: vec![*cid2.cid(), *cid4.cid(), *cid6.cid()] })
			.unwrap();

		let cids: Vec<Cid> = vec![&cid1, &cid2, &cid3, &cid4, &cid5, &cid6]
			.into_iter()
			.map(|block| *block.cid())
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
		let (storage, cids) = get_reference_storage().await.expect("storage");
		let resolver = create_cid_resolver(vec![storage]).await.expect("resovlers");
		let result = MultiLayerCidResolver::new().resolve_cid(cids.first().unwrap(), &resolver).await;
		assert_eq!(result.new_cid_map.len(), 6);
		assert_eq!(result.added_cids.len(), 6);
		for cid in cids.iter() {
			assert!(result.added_cids.contains(cid));
		}
	}

	#[tokio::test]
	async fn change_cid5_name() {
		// reference
		let (mut storage, cids) = get_reference_storage().await.expect("storage");
		let resolver = create_cid_resolver(vec![storage.clone()]).await.expect("resovlers");
		let first_result = MultiLayerCidResolver::new().resolve_cid(cids.first().unwrap(), &resolver).await;

		let next_cids = get_reference_storage_v2(&mut storage).await.expect("next cids");
		let updated_resolver = create_cid_resolver(vec![storage]).await.expect("resovlers");
		let second_result = MultiLayerCidResolver::new()
			.with_previous_cids(first_result.new_cid_map)
			.resolve_cid(next_cids.first().unwrap(), &updated_resolver)
			.await;

		assert_eq!(second_result.added_cids.len(), 3);
		assert!(second_result.added_cids.contains(next_cids.first().unwrap()));
		assert!(second_result.added_cids.contains(next_cids.get(3).unwrap()));
		assert!(second_result.added_cids.contains(next_cids.get(4).unwrap()));
		assert_eq!(second_result.removed_cids.len(), 3);
		assert!(second_result.removed_cids.contains(cids.first().unwrap()));
		assert!(second_result.removed_cids.contains(cids.get(3).unwrap()));
		assert!(second_result.removed_cids.contains(cids.get(4).unwrap()));
	}
}
