// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
