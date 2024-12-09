use super::Command as FileCommand;
use crate::{
	cli::Cli,
	library::{cli_context::CliContext, file::list_nodes},
};
use chrono::DateTime;
use co_core_file::Node;
use co_primitives::{AbsolutePath, Date, PathExt};
use co_sdk::{CoReducerError, CoReducerFactory};
use exitcode::ExitCode;
use futures::TryStreamExt;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The path.
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
	let file_state = match co_reducer.state(&file_command.core).await {
		Err(CoReducerError::CoreNotFound(_)) => Ok(co_core_file::File::default()),
		result => result,
	}?;

	// nodes
	let path = AbsolutePath::from_str(&command.path)?.normalize()?;
	let nodes: Vec<Node> = list_nodes(co_reducer.storage(), file_state, path).try_collect().await?;

	// print
	println!("total {}", nodes.len());
	fn format_row(size: u64, modify_time: Date, name: &str) -> String {
		format!(" {}B {} {}", size, format_date(modify_time), name)
	}
	for node in nodes {
		match node {
			Node::Folder(f) => println!("{}", format_row(0, f.modify_time, &f.name)),
			Node::File(f) => println!("{}", format_row(f.size, f.modify_time, &f.name)),
			Node::Link(f) => println!("{} -> {}", format_row(0, 0, &f.name), f.contents),
		}
	}

	// result
	Ok(exitcode::OK)
}

fn format_date(date: Date) -> String {
	if let Some(time) = DateTime::from_timestamp((date / 1_000) as i64, (date % 1_000 * 1_000_000) as u32) {
		time.to_rfc3339()
	} else {
		format!("[{}]", date)
	}
}
