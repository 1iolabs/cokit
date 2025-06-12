use crate::{
	library::network_queue::{
		network_queue_action, network_queue_backlog, network_queue_message, network_queue_task_complete,
		network_queue_task_doing,
	},
	Action, CoContext, CoUuid,
};
use co_actor::Actions;
use co_identity::PrivateIdentity;
use co_primitives::CoTryStreamExt;
use futures::{FutureExt, Stream};

/// If no peers could be found to send a DidComm message to a Co put it in the queue.
pub fn network_queue_message_epic(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::CoDidCommSent { message, result: Ok(peers) }
			if !message.tags.contains_key("task_id") && peers.is_empty() =>
		{
			let context = context.clone();
			let message = message.clone();
			Some(
				async move { network_queue_message(&context, message).await }
					.into_stream()
					.try_ignore_elements(),
			)
		},
		_ => None,
	}
}

/// When network has started try to process pending messages.
pub fn network_started_epic(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	_context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkStarted => {
			Some(async move { Ok(Action::NetworkQueueProcess { co: Default::default() }) }.into_stream())
		},
		_ => None,
	}
}

/// Process board tasks.
/// TODO: Only run once.
/// TODO: Concurrency.
/// TODO: On error clear task locks.
/// TODO: Add more triggers. Timeout? Backoff?
pub fn network_queue_process_epic(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkQueueProcess { co } => Some({
			let co = co.clone();
			let actions = actions.clone();
			let context = context.clone();
			let lock = context.uuid().uuid();
			async_stream::try_stream! {
				let local_co = context.local_co_reducer().await?;
				let identity = context.local_identity().boxed();
				let tasks = network_queue_backlog(context.clone(), {
					let co = co.clone();
					move |task| {
						if let Some(co) = &co {
							return task.tags.string("co") == Some(co.as_str());
						} else {
							true
						}
					}
				});
				for await task in tasks {
					let task = task?;
					let (action, complete) = network_queue_action(&local_co, &task, &lock).await?;

					// doing
					if !network_queue_task_doing(&identity, &local_co, &task, &lock).await? {
						continue;
					}

					// register complete
					let complete_fut = actions.once_map(complete);

					// send
					yield action;

					// wait complete
					let task_state = complete_fut.await?;

					// move task
					network_queue_task_complete(&identity, &local_co, &task, &lock, task_state).await?;
				}
			}
		}),
		_ => None,
	}
}
