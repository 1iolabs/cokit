use crate::{cli::Cli, library::application::application};
use anyhow::anyhow;
use co_sdk::{BlockStorage, MultiCodec};
use exitcode::ExitCode;
use libipld::{cbor::DagCborCodec, codec::Codec, Cid, Ipld};
use std::{io::Write, str::FromStr};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
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
			println!("Codec: {:?} ({})", Into::<MultiCodec>::into(cid.codec()), cid.codec());
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
