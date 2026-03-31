// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::Command as FileCommand;
use crate::{
	cli::Cli,
	library::{
		cli_context::CliContext,
		file::{file_core, get_nodes, FileError},
	},
};
use anyhow::anyhow;
use co_core_file::{FileAction, FileNode, Node};
use co_primitives::{tags, AbsolutePath, PathExt};
use co_sdk::{CoReducerFactory, Identity};
use co_storage::unixfs_add_file;
use exitcode::ExitCode;
use std::{os::unix::fs::MetadataExt, time::UNIX_EPOCH};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The path.
	pub path: String,

	/// The local file path.
	pub file_path: String,
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
	let (storage, file_state) = file_core(co_reducer.clone(), &identity, &file_command.core).await?;

	// path
	let path = AbsolutePath::from_str(&command.path)?.normalize()?;
	let (parent_path, name) = path.parent_and_file_name_result()?;

	// validate
	//  test if parent path exists
	get_nodes(storage.clone(), file_state, vec![parent_path.to_owned()].into_iter().collect())
		.await?
		.get(parent_path)
		.ok_or_else(|| FileError::NoEntry(parent_path.into(), anyhow!("add")))?;

	// stat
	let stat = tokio::fs::metadata(&command.file_path).await?;

	// content
	let contents = unixfs_add_file(&storage, &command.file_path).await?;

	// action
	let action = FileAction::Create {
		path: parent_path.to_owned(),
		node: Node::File(FileNode {
			name: name.to_owned(),
			create_time: stat.created()?.duration_since(UNIX_EPOCH)?.as_millis() as u64,
			modify_time: stat.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as u64,
			tags: tags!(),
			owner: identity.identity().to_owned(),
			mode: stat.mode(),
			size: stat.size(),
			contents,
		}),
		recursive: false,
	};
	co_reducer.push(&identity, &file_command.core, &action).await?;

	// result
	Ok(exitcode::OK)
}
