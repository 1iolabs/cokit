use anyhow::anyhow;
use async_trait::async_trait;
use co_api::{Cid, Metadata};
use co_storage::BlockStorage;
use libipld::{cbor::DagCborCodec, prelude::Codec, serde::from_ipld, Ipld};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[async_trait]
pub trait CidResolver {
	async fn resolve(&self, cid: &Cid, ignorable_cids: &BTreeSet<&Cid>) -> Result<BTreeSet<Cid>, anyhow::Error>;
}

pub type CidResolverBox = Box<dyn CidResolver + Send + Sync + 'static>;

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
			Ipld::List(list) =>
				for ipld_inner in list {
					IpldResolver::<S>::get_next_cids(ipld_inner, new_cids, ignorable_cids);
				},
			// found map -> traverse as links might be contained
			Ipld::Map(map) => {
				let external = get_external(ipld).unwrap_or_default();
				for (k, i) in map.iter() {
					if !external.contains(k) {
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
					let ipld: Ipld = DagCborCodec::default().decode(block.data())?;
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
			match resolver.resolve(cid, ignorable_cids).await {
				Ok(result) => return Ok(result),
				Err(_) => (),
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

/**
 * Resolves a cid. Then Looks for other cids and tries to recursively resolve those as well.
 * Will fail when a cid resolves to another Co as the given storage doesn't have its key.
 */
pub struct MultiLayerCidResolver {
	/// Contains all cids that have been found after running
	new_cids: BTreeMap<Cid, BTreeSet<Cid>>,
	/// Contains all cids that couldn't be resolved further
	failed_cids: BTreeSet<Cid>,
	/// Optional cid map from a previous state. Will simultaneously run a diff when this is set
	previous_cids: Option<BTreeMap<Cid, BTreeSet<Cid>>>,
	/// Defines a depth up to which Links should be resolved. No limit if depth is negtive
	depth_limit: i64,
	/// Tracks depth. After running, shows how many layers of links got resolved.
	current_depth: i64,
	/// After running, contains all cids that were not in the previous map but in the new one
	added_cids: BTreeSet<Cid>,
	/// Flag to check if resolver was executed
	ran: bool,
	/// Internally used
	discovered_cids: BTreeSet<Cid>,
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
			ran: false,
		}
	}

	/// implements a depth limit for calculations. Limit should never be set for real calculations
	pub fn with_depth_limit(mut self, depth: i64) -> Self {
		self.depth_limit = depth;
		self
	}
	/// additionally calculates which cids are left over from the previous cid map
	pub fn with_previous_cids(mut self, previous_cids: BTreeMap<Cid, BTreeSet<Cid>>) -> Self {
		self.previous_cids = Some(previous_cids);
		self
	}

	/// Returns a tuple (reached_depth, depth_limit) or an error if resolver wasn't executed
	pub fn depth(&self) -> anyhow::Result<(i64, i64)> {
		if self.ran {
			return Ok((self.current_depth, self.depth_limit));
		}
		Err(anyhow!("Resolver must be executed before results can be returned"))
	}

	/// Returns a tuple (removed_cids, added_cids) or an error if resolver wasn't executed
	pub fn diff(&self) -> anyhow::Result<(BTreeSet<Cid>, BTreeSet<Cid>)> {
		if self.ran {
			match &self.previous_cids {
				Some(p) => return Ok((p.keys().cloned().collect(), self.added_cids.clone())),
				None => return Err(anyhow!("No previous cids were set so no diff was calculated")),
			}
		}
		Err(anyhow!("Resolver must be executed before results can be returned"))
	}

	/// Returns the new cid map or an error if the resolver didn't run
	pub fn new_cids(&self) -> anyhow::Result<BTreeMap<Cid, BTreeSet<Cid>>> {
		if self.ran {
			return Ok(self.new_cids.clone());
		}
		Err(anyhow!("Resolver must be executed before results can be returned"))
	}

	/// Returns all failed cids if resolver ran
	pub fn failed_cids(&self) -> anyhow::Result<BTreeSet<Cid>> {
		if self.ran {
			return Ok(self.failed_cids.clone());
		}
		Err(anyhow!("Resolver must be executed before results can be returned"))
	}

	/// resolves a cid using the given resolvers. This will destruct data so you should run this twice
	pub async fn resolve_cid(&mut self, cid: &Cid, resolver: &CidResolverBox) -> anyhow::Result<()> {
		// check if first execution
		if self.ran {
			return Err(anyhow!("Resolver already ran"));
		}
		self.discovered_cids.insert(*cid);
		// resolve cids as long as there are new ones
		while self.discovered_cids.len() > 0 {
			// check if we reached defined depth
			if self.depth_limit >= 0 && self.depth_limit <= self.current_depth {
				break;
			} else {
				self.current_depth += 1;
			}
			// copy new cids for this iteration (to not iterate over a mutable set)
			let new_cids = self.discovered_cids.clone();
			// move all new cids to added set
			self.added_cids.append(&mut self.discovered_cids);
			for new_cid in new_cids.iter() {
				match &mut self.previous_cids {
					Some(p) =>
						if !move_entry(p, &mut self.new_cids, new_cid) {
							self.resolve(new_cid, resolver).await;
						},
					None => {
						self.resolve(new_cid, resolver).await;
					},
				}
			}
		}
		// set that resolver ran successfully
		self.ran = true;
		Ok(())
	}
	async fn resolve(&mut self, cid: &Cid, resolver: &CidResolverBox) {
		if let Ok(mut links) = resolver.resolve(cid, &self.new_cids.keys().collect()).await {
			self.new_cids.insert(cid.clone(), links.clone());
			self.discovered_cids.append(&mut links);
			self.failed_cids.remove(&cid.clone());
		} else {
			self.failed_cids.insert(cid.clone());
		}
	}
}

/// Try to move an entry and all its children.
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
