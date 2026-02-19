// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	cli::Cli,
	library::{
		cat::{cat_output, CatOptions},
		cli_context::CliContext,
	},
};
use anyhow::anyhow;
use cid::Cid;
use co_sdk::{BlockStorage, CoId, CoReducerFactory};
use exitcode::ExitCode;
use std::str::FromStr;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// The CID to print.
	/// If not specified using the root state.
	/// To get deeper the whole path of CIDs needs to be specified.
	pub cid: Vec<String>,

	/// Pretty print data.
	#[arg(short, long)]
	pub pretty: bool,

	/// Print useing formatter.
	#[arg(long)]
	pub format: Option<String>,

	/// Skip decrypt block if Cid is encrypted.
	#[arg(short, long)]
	pub no_decrypt: bool,
}

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	// reducer
	let application = context.application(cli).await;
	let reducer = application.context().try_co_reducer(&command.co).await?;
	let storage = reducer.storage();

	// cid
	//  we need to walk the path to have the mappings available
	//  for convenience we also dudup same cids
	let mut cid = reducer.reducer_state().await.0.ok_or(anyhow!("CO is empty"))?;
	let mut last_cid = None;
	let cid_len = command.cid.len();
	for (cid_index, next_cid) in [Ok(cid)]
		.into_iter()
		.chain(command.cid.iter().map(|cid_str| Cid::from_str(cid_str)))
		.enumerate()
	{
		let next_cid = next_cid?;
		if last_cid == Some(next_cid) {
			continue;
		}
		let _stat = storage.stat(&next_cid).await?;
		cid = next_cid;
		last_cid = Some(cid);
		if command.pretty {
			if cid_index == cid_len {
				println!("Cid: {}", cid);
			} else {
				println!("Parent Cid: {}", cid);
			}
		}
	}

	// print
	cat_output(
		reducer.storage(),
		cid,
		CatOptions::default()
			.with_pretty(command.pretty)
			.with_decrypt(!command.no_decrypt)
			.with_format(command.format.clone()),
	)
	.await?;

	// result
	Ok(exitcode::OK)
}
