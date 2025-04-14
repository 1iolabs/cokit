use super::Command as FileCommand;
use crate::{
	cli::Cli,
	library::{
		cat::cat_output,
		cli_context::CliContext,
		file::{get_nodes, FileError},
	},
};
use anyhow::anyhow;
use cid::Cid;
use co_core_file::{File, Node};
use co_primitives::{AbsolutePath, AbsolutePathOwned, PathExt};
use co_sdk::{
	state::{query_core, QueryExt},
	CoReducerFactory, CoStorage,
};
use exitcode::ExitCode;
use futures::{future::BoxFuture, FutureExt};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The path.#
	pub path: String,

	/// Pretty print data.
	#[arg(short, long)]
	pub pretty: bool,
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	file_command: &FileCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let co_reducer = application.context().try_co_reducer(&file_command.co).await?;
	let (storage, file_state) = query_core::<File>(&file_command.core)
		.with_default()
		.execute_reducer(&co_reducer)
		.await?;

	// contents
	let path = AbsolutePath::from_str(&command.path)?.normalize()?;
	let content = node_cid(storage.clone(), file_state, path).await?;

	// print
	cat_output(storage, content, command.pretty).await?;

	// result
	Ok(exitcode::OK)
}

fn node_cid(
	storage: CoStorage,
	file_state: co_core_file::File,
	path: AbsolutePathOwned,
) -> BoxFuture<'static, Result<Cid, FileError>> {
	async move {
		let path = path.normalize()?;
		let node = get_nodes(storage.clone(), file_state.clone(), vec![path.clone()].into_iter().collect())
			.await?
			.remove(&path)
			.ok_or(FileError::NoEntry(path.clone().into(), anyhow!("nodes")))?;
		let content = match node {
			Node::Folder(_) => Err(FileError::NoFile(path.clone().into(), anyhow!("folder")))?,
			Node::File(file) => file.contents,
			Node::Link(link) => {
				let parent = path
					.parent()
					.ok_or(FileError::NoEntry(path.clone().into(), anyhow!("parent")))?;
				node_cid(storage, file_state, parent.join(&link.contents)?).await?
			},
		};
		Ok(content)
	}
	.boxed()
}
