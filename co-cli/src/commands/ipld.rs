// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::library::cli_context::CliContext;
use cid::{Cid, Version};
use co_primitives::{from_cbor, KnownMultiCodec};
use co_sdk::MultiCodec;
use exitcode::ExitCode;
use ipld_core::ipld::Ipld;
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
	let cid = parse_cid(&command.cid)?;
	if cid.version() == Version::V1 {
		if KnownMultiCodec::DagPb == cid.codec() {
			if let Ok(v0) = Cid::new_v0(*cid.hash()) {
				println!("V0: {}", v0);
			}
		}
		println!("V1: {} (specified)", cid);
	} else if cid.version() == Version::V0 {
		println!("V0: {} (specified)", cid);
		println!("V1: {}", Cid::new_v1(cid.codec(), *cid.hash()));
	}
	println!("Codec: {} (code={})", MultiCodec::from(cid.codec()), cid.codec());
	println!("Hash {} (code={}, size={}):", MultiCodec::from(cid.hash().code()), cid.hash().code(), cid.hash().size());
	hexdump::hexdump(cid.hash().digest());
	Ok(exitcode::OK)
}

fn parse_cid(cid: &str) -> Result<Cid, anyhow::Error> {
	// try as JSON array with ints as bytes
	// Like: `[18, 32, 39, 249, 200, 182, 10, ..., 125, 206, 29]`
	if cid.starts_with('[') {
		if let Ok(binary) = serde_json::from_str::<Vec<u8>>(cid) {
			if let Ok(cid) = Cid::try_from(binary) {
				return Ok(cid);
			}
		}
	}
	Ok(Cid::from_str(cid)?)
}
