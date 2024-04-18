use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::anyhow;
use co_api::{Cid, CoId, DagCollection};
use co_sdk::{BlockStorage, CoStorage, CO_CORE_NAME_PIN};
use colored::{ColoredString, Colorize};
use exitcode::ExitCode;
use libipld::{cbor::DagCborCodec, codec::Codec, Ipld};
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

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &command.command {
		Commands::Ls(list_command) => list_pins(context, cli, list_command).await,
		Commands::Gen(gen_command) => generate_pins(context, cli, gen_command).await,
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
	// get state of local co
	let application = context.application(cli).await;
	let co_reducer = application.co_reducer(&command.co).await?.ok_or(anyhow!("Co not found"))?;
	let storage = co_reducer.storage();
	let local_state = co_reducer.reducer_state().await.0;
	if let Some(local_state) = local_state {
		// generate cids up to depth
		let mut ipld_resolver = IpldResolver::new(&storage, command.depth);
		ipld_resolver.resolve_cid(&local_state).await?;
		for cid in ipld_resolver.found_cids {
			let mut cid_string: ColoredString = cid.to_string().white();
			if ipld_resolver.failed_cids.contains(&cid) {
				cid_string = cid_string.red();
			}
			println!("{}", cid_string);
		}
		if ipld_resolver.depth < 0 {
			println!("Looked in unlimited depth and got to {}", ipld_resolver.current_depth);
		} else {
			println!("Looked up to depth {} and got to {}", ipld_resolver.depth, ipld_resolver.current_depth);
		}
	}
	Ok(exitcode::OK)
}

/**
 * Resolves a cid. Then Looks for other cids and tries to recursively resolve those as well.
 * Will fail when a cid resolves to another Co as the given storage doesn't have its key.
 */
struct IpldResolver<'a> {
	/// Contains all cids that have been found after running
	pub found_cids: BTreeSet<Cid>,
	/// Contains all cids that couldn't be resolved further
	pub failed_cids: BTreeSet<Cid>,
	pub storage: &'a CoStorage,
	/// Defines a depth up to which Links should be resolved. No limit if depth is negtive
	pub depth: i64,
	/// Tracks depth. After running, shows how many layers of links got resolved.
	pub current_depth: i64,
	new_cids: BTreeSet<Cid>,
}

impl<'a> IpldResolver<'a> {
	pub fn new(storage: &'a CoStorage, depth: i64) -> Self {
		Self {
			found_cids: BTreeSet::new(),
			new_cids: BTreeSet::new(),
			failed_cids: BTreeSet::new(),
			storage,
			depth,
			current_depth: 0,
		}
	}
	pub async fn resolve_cid(&mut self, cid: &Cid) -> Result<(), anyhow::Error> {
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
				self.found_cids.insert(new_cid.clone());
				// try to resolve. This can fail when cid is of another CO so we need to catch this error here
				if let Ok(ipld) = self.get_ipld(&new_cid).await {
					self.get_next_cids(&ipld);
				} else {
					self.failed_cids.insert(new_cid.clone());
				}
			}
		}
		Ok(())
	}
	async fn get_ipld(&self, cid: &Cid) -> Result<Ipld, anyhow::Error> {
		let block = self.storage.get(cid).await?;
		let ipld: Ipld = DagCborCodec::default().decode(block.data())?;
		Ok(ipld)
	}
	pub fn get_next_cids(&mut self, ipld: &Ipld) {
		match ipld {
			// found cid -> add to list of new cids
			Ipld::Link(cid) => {
				self.new_cids.insert(*cid);
			},
			// found list -> traverse as links might be contained
			Ipld::List(list) =>
				for ipld_inner in list {
					self.get_next_cids(ipld_inner);
				},
			// found map -> traverse as links might be contained
			Ipld::Map(map) =>
				for ipld_inner in map.values() {
					self.get_next_cids(ipld_inner);
				},
			// No need to check further as no other types can contain links
			_ => (),
		};
	}
}
