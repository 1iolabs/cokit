use crate::{cli::Cli, library::application::application};
use anyhow::anyhow;
use co_sdk::{memberships, BlockStorage, MultiCodec};
use exitcode::ExitCode;
use futures::{pin_mut, stream::StreamExt};
use libipld::{cbor::DagCborCodec, codec::Codec, Cid, Ipld};
use std::{io::Write, str::FromStr};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO Command
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// List all local COs.
	Ls,

	/// Print a block.
	Cat(CatCommand),
}

#[derive(Debug, Clone, clap::Args)]
pub struct CatCommand {
	/// CO ID
	pub co: String,

	/// The CID to print.
	/// If not specified using the root state.
	pub cid: Option<String>,

	/// Pretty print data.
	#[arg(short, long)]
	pub pretty: bool,
}

pub async fn command(cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &command.command {
		Commands::Ls => ls(cli).await,
		Commands::Cat(cat_command) => cat(cli, cat_command).await,
	}
}

async fn ls(cli: &Cli) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;
	let local_co_reducer = application.local_co_reducer().await?;

	// list
	let mut result = exitcode::OK;
	let stream = memberships(local_co_reducer.clone());
	pin_mut!(stream);
	while let Some(item) = stream.next().await {
		match item {
			Ok((id, state, tags)) => {
				println!("{} | {} | {}", id, state.to_string(), tags)
			},
			Err(e) => {
				result = exitcode::UNAVAILABLE;
				eprintln!("error: {:?}", e);
			},
		}
	}

	// result
	Ok(result)
}

async fn cat(cli: &Cli, command: &CatCommand) -> Result<ExitCode, anyhow::Error> {
	// reducer
	let application = application(cli).await?;
	let reducer = application
		.co_reducer(&command.co)
		.await?
		.ok_or(anyhow!("Co not found: {}", command.co))?;

	// cid
	let cid = match &command.cid {
		Some(cid) => Cid::from_str(cid)?,
		None => reducer.reducer_state().await.0.ok_or(anyhow!("CO is empty"))?,
	};

	// block
	let block = reducer.storage().get(&cid).await?;

	// output
	if command.pretty {
		if MultiCodec::CoEncryptedBlock == cid.codec().into() {
			println!("Cid: {}", block.cid());
		}
		let codec = MultiCodec::from(block.cid().codec());
		println!("Codec: {:?} ({})", codec, block.cid().codec());
		println!("Size: {}", block.data().len());
		match codec {
			MultiCodec::DagCbor => {
				let ipld: Ipld = DagCborCodec::default().decode(block.data())?;
				println!("{:#?}", ipld);
			},
			_ => {
				hexdump::hexdump(block.data());
			},
		}
	} else {
		let mut out = std::io::stdout();
		out.write_all(block.data())?;
		out.flush()?;
	}

	// result
	Ok(exitcode::OK)
}
