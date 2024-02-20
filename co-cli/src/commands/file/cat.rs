use super::Command as FileCommand;
use crate::{
	cli::Cli,
	library::{
		application::application,
		cat::cat_output,
		file::{get_nodes, FileError},
	},
};
use anyhow::anyhow;
use co_core_file::Node;
use co_primitives::{AbsolutePath, AbsolutePathOwned, PathExt};
use co_sdk::{CoReducerError, CoStorage};
use exitcode::ExitCode;
use futures::{future::BoxFuture, FutureExt};
use libipld::Cid;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The path.#
	pub path: String,

	/// Pretty print data.
	#[arg(short, long)]
	pub pretty: bool,
}

pub async fn command(cli: &Cli, file_command: &FileCommand, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;
	let co_reducer = application
		.co_reducer(&file_command.co)
		.await?
		.ok_or(anyhow!("Co not found: {}", file_command.co))?;
	let file_state = match co_reducer.state(&file_command.core).await {
		Err(CoReducerError::CoreNotFound(_)) => Ok(co_core_file::File::default()),
		result => result,
	}?;

	// contents
	let path = AbsolutePath::from_str(&command.path)?.normalize()?;
	let content = node_cid(co_reducer.storage(), file_state, path).await?;

	// print
	cat_output(co_reducer.storage(), content, command.pretty).await?;

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
