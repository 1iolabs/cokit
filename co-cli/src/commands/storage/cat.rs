use crate::{
	cli::Cli,
	library::{application::application, cat::cat_output},
};
use co_primitives::Secret;
use co_sdk::CoStorage;
use co_storage::EncryptedBlockStorage;
use exitcode::ExitCode;
use libipld::Cid;
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The CID to print.
	/// If not specified using the root state.
	pub cid: String,

	/// Pretty print data.
	#[arg(short, long)]
	pub pretty: bool,

	/// Load encryption key from file.
	#[arg(short, long)]
	pub key_file: Option<PathBuf>,
}

pub async fn command(cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	// reducer
	let application = application(cli).await?;
	let mut storage = application.storage();

	// encryption?
	if let Some(key_file) = &command.key_file {
		let key_data: Vec<u8> = tokio::fs::read(key_file).await?;
		let key: Secret = serde_ipld_dagcbor::from_slice(&key_data)?;
		storage = CoStorage::new(EncryptedBlockStorage::new(storage, key.into(), Default::default()));
	}

	// print
	cat_output(storage, Cid::from_str(&command.cid)?, command.pretty).await?;

	// result
	Ok(exitcode::OK)
}
