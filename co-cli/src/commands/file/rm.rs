use super::Command as FileCommand;
use crate::{
	cli::Cli,
	library::{
		cli_context::CliContext,
		file::{file_core, get_nodes, FileError},
	},
};
use anyhow::anyhow;
use co_core_file::FileAction;
use co_primitives::{AbsolutePath, PathExt};
use co_sdk::CoReducerFactory;
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The path.
	pub path: String,

	/// Recursively delete.
	#[arg(short)]
	pub recursive: bool,
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	file_command: &FileCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let co_reducer = application.context().try_co_reducer(&file_command.co).await?;
	let identity = application.local_identity();

	// state
	let file_state: co_core_file::File = file_core(co_reducer.clone(), &identity, &file_command.core).await?;

	// path
	let path = AbsolutePath::from_str(&command.path)?.normalize()?;

	// validate
	// test if parent path exists
	get_nodes(co_reducer.storage(), file_state, vec![path.to_owned()].into_iter().collect())
		.await?
		.get(&path)
		.ok_or_else(|| FileError::NoEntry(path.clone().into(), anyhow!("rm")))?;

	// action
	let action = FileAction::Remove { path, recursive: command.recursive };
	co_reducer.push(&identity, &file_command.core, &action).await?;

	// result
	Ok(exitcode::OK)
}
