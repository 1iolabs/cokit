use crate::{
	cli::Cli,
	library::{cat::cat_output, cli_context::CliContext},
};
use anyhow::anyhow;
use cid::Cid;
use co_sdk::{CoId, CoReducerFactory};
use exitcode::ExitCode;
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

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	// reducer
	let application = context.application(cli).await;
	let reducer = application.context().try_co_reducer(&command.co).await?;

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
