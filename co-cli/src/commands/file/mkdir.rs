use super::Command as FileCommand;
use crate::{
	cli::Cli,
	library::{
		cli_context::CliContext,
		file::{file_core, get_nodes, FileError},
	},
};
use anyhow::anyhow;
use co_core_file::{FileAction, FolderNode, Node};
use co_primitives::{tags, AbsolutePath, PathExt};
use co_sdk::{CoReducerFactory, Identity};
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The path.
	pub path: String,

	/// Recursively create path.
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
	let (parent_path, name) = path.parent_and_file_name_result()?;

	// validate
	if !command.recursive {
		// test if parent path exists
		get_nodes(co_reducer.storage(), file_state, vec![parent_path.to_owned()].into_iter().collect())
			.await?
			.get(parent_path)
			.ok_or_else(|| FileError::NoEntry(parent_path.into(), anyhow!("mkdir")))?;
	}

	// action
	let action = FileAction::Create {
		path: parent_path.to_owned(),
		node: Node::Folder(FolderNode {
			name: name.to_owned(),
			create_time: 0,
			modify_time: 0,
			tags: tags!(),
			owner: identity.identity().to_owned(),
			mode: 0o665,
		}),
		recursive: command.recursive,
	};
	co_reducer.push(&identity, &file_command.core, &action).await?;

	// result
	Ok(exitcode::OK)
}
