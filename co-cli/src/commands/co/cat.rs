use crate::{
	cli::Cli,
	library::{application::application, cat::cat_output},
};
use anyhow::anyhow;
use co_sdk::CoId;
use exitcode::ExitCode;
use libipld::Cid;
use std::str::FromStr;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

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

	// print
	cat_output(reducer.storage(), cid, command.pretty).await?;

	// result
	Ok(exitcode::OK)
}
