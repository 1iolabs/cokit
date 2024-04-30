use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::anyhow;
use async_trait::async_trait;
use co_api::{Cid, CoId, DagCollection, DefaultNodeSerializer, NodeBuilder, StorageError};
use co_sdk::{memberships, BlockStorage, CoStorage, NodeStream, OptionLink, CO_CORE_NAME_PIN};
use colored::Colorize;
use exitcode::ExitCode;
use futures::{pin_mut, StreamExt};
use libipld::{cbor::DagCborCodec, codec::Codec, Ipld};
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Pin testing commands
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// Lists cids that are manually pinned
	Ls(ListCommand),
	/// Generates pins by traversing state
	Gen(GenerateCommand),
	/// Updates the pin map for auto state pinning
	Update(UpdateCommand),
}

#[derive(Debug, Clone, clap::Args)]
pub struct ListCommand {
	/// Sums up the number of pins
	#[arg(short, long, default_value_t = false)]
	pub sum: bool,
	/// Includes tags when printing
	#[arg(short, long, default_value_t = false)]
	pub all: bool,
	/// Lists all pins
	#[arg(short, long, default_value_t = false)]
	pub list: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct GenerateCommand {
	/// depth to generate pinned cids to
	#[arg(short, long, default_value_t = -1)]
	pub depth: i64,
	/// co id
	#[arg(short, long, default_value_t = CoId::new("local"))]
	pub co: CoId,
}

#[derive(Debug, Clone, clap::Args)]
pub struct UpdateCommand {}

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &command.command {
		Commands::Ls(list_command) => list_pins(context, cli, list_command).await,
		Commands::Gen(gen_command) => generate_pins(context, cli, gen_command).await,
		Commands::Update(update_command) => update_pins(context, cli, update_command).await,
	}
}

pub async fn list_pins(context: &CliContext, cli: &Cli, command: &ListCommand) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;

	let local_co_reducer = application.local_co_reducer().await?;
	let storage = local_co_reducer.storage();
	let pin_state = local_co_reducer.state::<co_core_pin::Pin>(CO_CORE_NAME_PIN).await?;
	if let Some(link) = pin_state.pins.link().cid() {
		let block = storage.get(link).await?;

		let map: BTreeMap<String, Vec<Vec<Cid>>> = DagCborCodec::default().decode(block.data())?;
		if let Some(inner) = map.get("l") {
			if command.sum {
				println!("Total number of current pins: {}", inner.len());
			}
			if command.sum && command.list {
				// get terminal width
				let (x, _y) = termion::terminal_size().unwrap();
				// hline
				println!("{:-<width$}", "-", width = x as usize);
			}
			if command.list {
				for cid_pair in inner.iter() {
					if command.all {
						let block = storage.get(&cid_pair[1]).await?;
						let tags: BTreeMap<String, Ipld> = DagCborCodec::default().decode(block.data())?;
						let tags_pretty = tags.get("l").expect("non empty");
						println!("Cid {} pinned by tags:\n\t {:?}", cid_pair[0].to_string(), tags_pretty);
					} else {
						println!("{}", cid_pair[0].to_string());
					}
				}
			}
		}
	}
	Ok(exitcode::OK)
}

pub async fn generate_pins(
	context: &CliContext,
	cli: &Cli,
	command: &GenerateCommand,
) -> Result<ExitCode, anyhow::Error> {
	// get state of given co
	let application = context.application(cli).await;
	let co_reducer = application.co_reducer(&command.co).await?.ok_or(anyhow!("Co not found"))?;
	let storage = co_reducer.storage();
	let state = co_reducer.reducer_state().await.0;

	if let Some(state) = state {
		// generate cids up to depth
		let mut ipld_resolver = MultiLayerCidResolver::new(command.depth);
		ipld_resolver.resolve_cid(&state, &storage, &create_cid_resolver()).await?;

		// print findings
		for (cid, children) in ipld_resolver.found_cids {
			// print found cid
			println!("Cid: {}", cid.to_string());

			// print all children
			for child in children {
				let mut child_string = child.to_string().bright_white();
				// mark child if cid could not be resolved
				if ipld_resolver.failed_cids.contains(&child) {
					child_string = child_string.red();
				}
				println!("\t{}", child_string);
			}
		}

		// print depth info
		if ipld_resolver.depth < 0 {
			println!("Looked in unlimited depth and got to {}", ipld_resolver.current_depth);
		} else {
			println!("Looked up to depth {} and got to {}", ipld_resolver.depth, ipld_resolver.current_depth);
		}
	}
	Ok(exitcode::OK)
}

async fn update_pins(context: &CliContext, cli: &Cli, _command: &UpdateCommand) -> Result<ExitCode, anyhow::Error> {
	// application ini
	let application = context.application(cli).await;

	// stream over all CO memberships
	let stream = memberships(application.local_co_reducer().await?);
	pin_mut!(stream);
	let mut ipld_resolver = MultiLayerCidResolver::new(-1);
	while let Some(item) = stream.next().await {
		match item {
			Ok((co, cid, _tags)) => {
				// for current CO, resolve state cid
				let reducer = application.co_reducer(co).await?.ok_or(anyhow!("Co not found"))?;
				let storage = reducer.storage();
				ipld_resolver.resolve_cid(&cid, &storage, &create_cid_resolver()).await?;
			},
			Err(e) => {
				eprintln!("error: {:?}", e);
			},
		}
	}

	// get all resolved cids
	let _found_cids = ipld_resolver.found_cids;

	// TODO write pin map

	Ok(exitcode::OK)
}

#[async_trait]
pub trait CidResolver {
	async fn resolve(
		&self,
		storage: &CoStorage,
		cid: &Cid,
		ignorable_cids: &BTreeSet<&Cid>,
	) -> Result<BTreeSet<Cid>, anyhow::Error>;
}

pub type CidResolverBox = Box<dyn CidResolver + Send + Sync + 'static>;

#[derive(Debug, Clone, Default)]
pub struct IpldResolver {}

impl IpldResolver {
	pub fn get_next_cids(&self, ipld: &Ipld, new_cids: &mut BTreeSet<Cid>, ignorable_cids: &BTreeSet<&Cid>) {
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
					self.get_next_cids(ipld_inner, new_cids, ignorable_cids);
				},
			// found map -> traverse as links might be contained
			Ipld::Map(map) =>
				for ipld_inner in map.values() {
					self.get_next_cids(ipld_inner, new_cids, ignorable_cids);
				},
			// No need to check further as no other types can contain links
			_ => (),
		};
	}
}

#[async_trait]
impl CidResolver for IpldResolver {
	async fn resolve(
		&self,
		storage: &CoStorage,
		cid: &Cid,
		ignorable_cids: &BTreeSet<&Cid>,
	) -> Result<BTreeSet<Cid>, anyhow::Error> {
		let block = storage.get(cid).await?;
		let ipld: Ipld = DagCborCodec::default().decode(block.data())?;
		let mut links: BTreeSet<Cid> = BTreeSet::new();
		self.get_next_cids(&ipld, &mut links, ignorable_cids);
		Ok(links)
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
	async fn resolve(
		&self,
		storage: &CoStorage,
		cid: &Cid,
		ignorable_cids: &BTreeSet<&Cid>,
	) -> Result<BTreeSet<Cid>, anyhow::Error> {
		for resolver in self.resolvers.iter() {
			match resolver.resolve(storage, cid, ignorable_cids).await {
				Ok(result) => return Ok(result),
				Err(_) => (),
			}
		}
		Err(anyhow!("couldn't resolve"))
	}
}

pub fn create_cid_resolver() -> CidResolverBox {
	Box::new(JoinCidResolver::new(vec![Box::new(IpldResolver {})]))
}

/**
 * Resolves a cid. Then Looks for other cids and tries to recursively resolve those as well.
 * Will fail when a cid resolves to another Co as the given storage doesn't have its key.
 */
struct MultiLayerCidResolver {
	/// Contains all cids that have been found after running
	pub found_cids: BTreeMap<Cid, BTreeSet<Cid>>,
	/// Contains all cids that couldn't be resolved further
	pub failed_cids: BTreeSet<Cid>,
	/// Defines a depth up to which Links should be resolved. No limit if depth is negtive
	pub depth: i64,
	/// Tracks depth. After running, shows how many layers of links got resolved.
	pub current_depth: i64,
	new_cids: BTreeSet<Cid>,
}

impl MultiLayerCidResolver {
	pub fn new(depth: i64) -> Self {
		Self {
			found_cids: BTreeMap::new(),
			new_cids: BTreeSet::new(),
			failed_cids: BTreeSet::new(),
			depth,
			current_depth: 0,
		}
	}
	pub async fn resolve_cid(
		&mut self,
		cid: &Cid,
		storage: &CoStorage,
		resolver: &CidResolverBox,
	) -> Result<(), anyhow::Error> {
		self.new_cids.insert(*cid);
		while self.new_cids.len() > 0 {
			// check if we reached defined depth
			if self.depth >= 0 && self.depth <= self.current_depth {
				break;
			} else {
				self.current_depth += 1;
			}
			let new_cids = self.new_cids.clone();
			self.new_cids.clear();
			for new_cid in new_cids.iter() {
				// try to resolve. This can fail when no given resolver can resolve this Cid
				if let Ok(mut links) = resolver.resolve(storage, new_cid, &self.found_cids.keys().collect()).await {
					self.found_cids.insert(new_cid.clone(), links.clone());
					self.new_cids.append(&mut links);
					self.failed_cids.remove(&new_cid.clone());
				} else {
					self.failed_cids.insert(new_cid.clone());
				}
			}
		}
		Ok(())
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
	async fn _read<S>(storage: S, cid: Cid) -> anyhow::Result<BTreeMap<Cid, BTreeSet<Cid>>>
	where
		S: BlockStorage + Sync + Send + Clone + 'static,
	{
		let mut result = BTreeMap::<Cid, BTreeSet<Cid>>::new();
		let mut root = None;
		let stream = NodeStream::from_link(storage, OptionLink::new(Some(cid)));
		pin_mut!(stream);
		while let Some(item) = stream.next().await {
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

	async fn _write<S: BlockStorage>(storage: &mut S, map: &BTreeMap<Cid, BTreeSet<Cid>>) -> anyhow::Result<Cid> {
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
		let result = blocks.get(0).expect("at least one block").cid().clone();
		for block in blocks.into_iter() {
			storage.set(block).await?;
		}

		// result
		Ok(result)
	}
}
