use crate::library::cli_context::CliContext;
use co_primitives::from_cbor;
use co_sdk::MultiCodec;
use exitcode::ExitCode;
use libipld::{Cid, Ipld};
use std::str::FromStr;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// IPLD Command
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// Print cbor from file.
	PrintCbor(PrintCborCommand),

	/// Inspect CID.
	InspectCid(InspectCidCommand),
}

#[derive(Debug, Clone, clap::Args)]
pub struct PrintCborCommand {
	/// The file to print.
	pub file: String,

	/// Pretty print data.
	#[arg(short, long)]
	pub pretty: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct InspectCidCommand {
	/// The CID to inspect.
	pub cid: String,
}

pub async fn command(_context: &CliContext, cbor: &Command) -> Result<ExitCode, anyhow::Error> {
	match &cbor.command {
		Commands::PrintCbor(command) => print(command).await,
		Commands::InspectCid(command) => inspect_cid(command).await,
	}
}

async fn print(command: &PrintCborCommand) -> Result<ExitCode, anyhow::Error> {
	let buf: Vec<u8> = tokio::fs::read(&command.file).await?;
	let ipld: Ipld = from_cbor(&buf)?;
	if command.pretty {
		println!("{:#?}", ipld);
	} else {
		println!("{:?}", ipld);
	}
	Ok(exitcode::OK)
}

async fn inspect_cid(command: &InspectCidCommand) -> Result<ExitCode, anyhow::Error> {
	let cid = Cid::from_str(&command.cid)?;
	println!("Version: {:?}", cid.version());
	println!("Codec: {} (code={})", MultiCodec::from(cid.codec()), cid.codec());
	println!("Hash {} (code={}, size={}):", MultiCodec::from(cid.hash().code()), cid.hash().code(), cid.hash().size());
	hexdump::hexdump(cid.hash().digest());
	Ok(exitcode::OK)
}
