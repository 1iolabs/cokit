use crate::{cli::Cli, library::application::application};
use anyhow::Ok;
use co_api::{Cid, DagCollection, Linkable};
use co_sdk::{BlockStorage, CO_CORE_NAME_PIN};
use exitcode::ExitCode;
use libipld::{cbor::DagCborCodec, codec::Codec, Ipld};
use std::{collections::BTreeMap, fmt::Debug};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Pin testing commands
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	Ls(ListCommand),
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

pub async fn command(cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &command.command {
		Commands::Ls(list_command) => list_pins(cli, list_command).await,
	}
}

pub async fn list_pins(cli: &Cli, command: &ListCommand) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;

	let local_co_reducer = application.local_co_reducer().await?;
	let storage = local_co_reducer.storage();
	let pin_state = local_co_reducer.state::<co_core_pin::Pin>(CO_CORE_NAME_PIN).await?;
	if let Some(link) = pin_state.pins.link() {
		let block = storage.get(link.cid()).await?;

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
						println!("Cid {} pinned by tags: {:?}", cid_pair[0].to_string(), tags_pretty);
					} else {
						println!("{}", cid_pair[0].to_string());
					}
				}
			}
		}
	}
	Ok(exitcode::OK)
}
