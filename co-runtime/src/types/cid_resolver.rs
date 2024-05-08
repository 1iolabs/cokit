use anyhow::anyhow;
use async_trait::async_trait;
use co_api::{Cid, Metadata};
use co_storage::BlockStorage;
use libipld::{cbor::DagCborCodec, prelude::Codec, serde::from_ipld, Ipld};
use std::collections::{BTreeSet, HashSet};

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
		Err(anyhow!("couldn't resolve"))
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
