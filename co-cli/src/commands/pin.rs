use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::anyhow;
use co_api::{Cid, CoId, DagCollection, DefaultNodeSerializer, NodeBuilder, StorageError};
use co_runtime::{create_cid_resolver, CidResolverBox};
use co_sdk::{memberships, Application, BlockStorage, CoStorage, NodeStream, OptionLink, CO_CORE_NAME_PIN};
use colored::Colorize;
use exitcode::ExitCode;
use futures::{pin_mut, StreamExt};
use libipld::{cbor::DagCborCodec, codec::Codec, Ipld};
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
};
use tokio::fs;

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

/// list function for all manual pins
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
	let state = co_reducer.reducer_state().await.0;

	if let Some(state) = state {
		// generate cids up to depth
		let mut resolver = MultiLayerCidResolver::new(command.depth);
		resolver
			.resolve_cid(&state, &create_cid_resolver(get_all_co_storages(application).await?).await?)
			.await?;

		// print findings
		cat_resolver(resolver, true);
	}
	Ok(exitcode::OK)
}

async fn update_pins(context: &CliContext, cli: &Cli, _command: &UpdateCommand) -> Result<ExitCode, anyhow::Error> {
	// application ini
	let application = context.application(cli).await;
	let pins_path = application.application_path().with_file_name("pins.cbor");
	let mut cid_resolver = MultiLayerCidResolver::new(-1);
	let (state, _) = application.local_co_reducer().await?.reducer_state().await;
	let resolvers = create_cid_resolver(get_all_co_storages(application).await?).await?;
	match state {
		Some(cid) => {
			cid_resolver.resolve_cid(&cid, &resolvers).await?;
		},
		None => (),
	}
	// TODO remove this and write unit test
	// test reading and printing saved pin map
	let content = fs::read(&pins_path).await?;
	let mut old_pin_map: BTreeMap<Cid, BTreeSet<Cid>> = serde_ipld_dagcbor::from_slice(&content)?;

	let mut new_pin_map = cid_resolver.found_cids.clone();
	// write pin map
	let data = serde_ipld_dagcbor::to_vec(&new_pin_map)?;
	fs::write(&pins_path, data).await?;

	let (removed_items, added_items) = pin_map_diff(&mut old_pin_map, &mut new_pin_map);
	println!("Removed items:");
	for i in removed_items {
		println!("{i}");
	}
	println!("Added items:");
	for i in added_items {
		println!("{i}");
	}
	cat_pin_map(old_pin_map, BTreeSet::default());

	Ok(exitcode::OK)
}

async fn get_all_co_storages(application: Application) -> anyhow::Result<Vec<CoStorage>> {
	let local_co_reducer = application.local_co_reducer().await?;
	let stream = memberships(local_co_reducer);
	let mut storages: Vec<CoStorage> = vec![];
	pin_mut!(stream);
	while let Some(result) = stream.next().await {
		match result {
			Ok((co, _, _)) =>
				if let Some(reducer) = application.co_reducer(co).await? {
					storages.push(reducer.storage());
				},
			Err(_) => (),
		}
	}
	Ok(storages)
}

/**
 * Resolves a cid. Then Looks for other cids and tries to recursively resolve those as well.
 * Will fail when a cid resolves to another Co as the given storage doesn't have its key.
 */
pub struct MultiLayerCidResolver {
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
	pub async fn resolve_cid(&mut self, cid: &Cid, resolver: &CidResolverBox) -> Result<(), anyhow::Error> {
		self.new_cids.insert(*cid);
		// resolve cids as long as there are new ones
		while self.new_cids.len() > 0 {
			// check if we reached defined depth
			if self.depth >= 0 && self.depth <= self.current_depth {
				break;
			} else {
				self.current_depth += 1;
			}
			// copy new cids for this iteration (to not iterate over a mutable set)
			let new_cids = self.new_cids.clone();
			self.new_cids.clear();
			for new_cid in new_cids.iter() {
				// try to resolve. This can fail when no given resolver can resolve this Cid
				if let Ok(mut links) = resolver.resolve(new_cid, &self.found_cids.keys().collect()).await {
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

/// takes two cid maps and returns a tuple (removed_items, added_items)
/// function is destructive and the 'old' map will only contain removed items and the 'new' map only added items after
/// running
pub fn pin_map_diff(
	old: &mut BTreeMap<Cid, BTreeSet<Cid>>,
	new: &mut BTreeMap<Cid, BTreeSet<Cid>>,
) -> (BTreeSet<Cid>, BTreeSet<Cid>) {
	for new_cid in new.clone().keys() {
		if old.contains_key(new_cid) {
			// cid has already been known.
			// Same cid -> content cannot have changed -> we can remove all children too
			cull_children(old, new_cid);
			cull_children(new, new_cid);
		}
	}
	(old.keys().cloned().collect(), new.keys().cloned().collect())
}

/// recursively removes entry with cid and all entries of it's children
fn cull_children(map: &mut BTreeMap<Cid, BTreeSet<Cid>>, cid: &Cid) {
	if let Some((_, children)) = map.remove_entry(cid) {
		for child in children {
			cull_children(map, &child);
		}
	}
}

pub fn cat_resolver(resolver: MultiLayerCidResolver, print_depth_info: bool) {
	// print information of found cid map
	cat_pin_map(resolver.found_cids, resolver.failed_cids);

	if print_depth_info {
		// print depth info
		if resolver.depth < 0 {
			println!("Looked in unlimited depth and got to {}", resolver.current_depth);
		} else {
			println!("Looked up to depth {} and got to {}", resolver.depth, resolver.current_depth);
		}
	}
}

pub fn cat_pin_map(found_cids: BTreeMap<Cid, BTreeSet<Cid>>, failed_cids: BTreeSet<Cid>) {
	for (cid, children) in found_cids {
		// print found cid
		println!("Cid: {}", cid.to_string());

		// print all children
		for child in children {
			let mut child_string = child.to_string().bright_white();
			// mark child if cid could not be resolved
			if failed_cids.contains(&child) {
				child_string = child_string.red();
			}
			println!("\t{}", child_string);
		}
	}
}
