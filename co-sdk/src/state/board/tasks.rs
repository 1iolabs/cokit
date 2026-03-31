// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{CoReducerState, CoStorage};
use co_core_board::{Board, Task};
use co_primitives::{CoTryStreamExt, LazyTransaction};
use co_storage::BlockStorageExt;
use futures::{Stream, TryStreamExt};
use std::future::ready;

/// Read board tasks from a list.
pub fn tasks(
	storage: CoStorage,
	reducer_state: CoReducerState,
	core: String,
	list_name: String,
) -> impl Stream<Item = Result<Task, anyhow::Error>> {
	async_stream::try_stream! {
		let co = storage.get_value_or_default(&reducer_state.co()).await?;
		if let Some(core) = co.cores.get(&core) {
			let board: Board = storage.get_default(&core.state).await?;
			let mut tasks = LazyTransaction::new(storage.clone(), board.tasks.clone());
			if let Some((_, list)) = board
				.lists
				.stream(&storage)
				.try_filter(|(_, list)| ready(list.name == list_name))
				.try_first()
				.await?
			{
				let task_ids = list.tasks.stream(&storage);
				for await task_id in task_ids {
					if let Some(task) = tasks.get().await?.get(&task_id?.1).await? {
						yield task;
					}
				}
			}
		}
	}
}
