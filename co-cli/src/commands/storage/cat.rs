// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	cli::Cli,
	library::{
		cat::{cat_output, CatOptions},
		cli_context::CliContext,
	},
};
use cid::Cid;
use co_primitives::{from_cbor, Secret};
use co_sdk::CoStorage;
use co_storage::EncryptedBlockStorage;
use exitcode::ExitCode;
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

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	// reducer
	let application = context.application(cli).await;
	let mut storage = application.storage();

	// encryption?
	if let Some(key_file) = &command.key_file {
		let key_data: Vec<u8> = tokio::fs::read(key_file).await?;
		let key: Secret = from_cbor(&key_data)?;
		storage =
			CoStorage::new(EncryptedBlockStorage::new(storage, key.into(), Default::default(), Default::default()));
	}

	// print
	cat_output(
		storage,
		Cid::from_str(&command.cid)?,
		CatOptions::default()
			.with_pretty(command.pretty)
			.with_decrypt(command.key_file.is_some()),
	)
	.await?;

	// result
	Ok(exitcode::OK)
}
