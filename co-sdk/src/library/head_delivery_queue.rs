// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#[cfg(test)]
use crate::library::network_queue::network_queue_tasks;
use crate::{
	library::{
		head_delivery::HeadsDeliveryIntent,
		network_queue::{ensure_network_queue_core, CO_CORE_NAME_NETWORK_QUEUE, LIST_NAME_BACKLOG},
	},
	CoContext, CoUuid,
};
use co_core_board::{BoardAction, Task, TaskId};
use co_primitives::{tags, CoId, Did, Tags, TagsExpr};
use co_storage::BlockStorageExt;
#[cfg(test)]
use futures::TryStreamExt;

pub(crate) const HEADS_DELIVERY_TASK_TYPE: &str = "co-heads-did";
const TAG_RECIPIENT: &str = "recipient";

pub(crate) fn head_task_selector(co: &CoId, recipient: &Did) -> TagsExpr {
	TagsExpr::new("task-type", HEADS_DELIVERY_TASK_TYPE)
		.and(TagsExpr::new("co", co.to_string()))
		.and(TagsExpr::new("recipient", recipient.to_string()))
}

pub(crate) fn head_task_selector_from_task(task: &Task) -> Option<TagsExpr> {
	let co = task.tags.string("co")?;
	let recipient = task.tags.string("recipient")?;
	(task.tags.string("task-type") == Some(HEADS_DELIVERY_TASK_TYPE))
		.then(|| head_task_selector(&CoId::from(co), &Did::from(recipient)))
}

fn task_name(intent: &HeadsDeliveryIntent) -> String {
	format!("Heads for {} to {}", intent.co, intent.recipient)
}

fn task_tags(intent: &HeadsDeliveryIntent) -> Tags {
	tags!(
		"co": intent.co.to_string(),
		"task-type": HEADS_DELIVERY_TASK_TYPE,
		TAG_RECIPIENT: intent.recipient.clone(),
	)
}

pub(crate) async fn enqueue_latest(context: &CoContext, intent: HeadsDeliveryIntent) -> Result<TaskId, anyhow::Error> {
	let local_co = context.local_co_reducer().await?;
	let identity = context.local_identity();
	let (storage, co) = local_co.co().await?;
	ensure_network_queue_core(context.cores(), &local_co, &identity, co).await?;

	let task_id = format!("co-heads-did-{}", context.uuid().uuid());
	let payload = Some(storage.set_serialized(&intent).await?);
	local_co
		.push(
			&identity,
			CO_CORE_NAME_NETWORK_QUEUE,
			&BoardAction::TaskEnqueueOrReplace {
				list: LIST_NAME_BACKLOG.to_owned(),
				matching: head_task_selector(&intent.co, &intent.recipient),
				task: Task {
					id: task_id.clone(),
					name: task_name(&intent),
					tags: task_tags(&intent),
					payload,
					lock: None,
				},
				after: None,
			},
		)
		.await?;
	Ok(task_id)
}

#[cfg(test)]
pub(crate) async fn head_tasks(
	context: &CoContext,
	list_name: &str,
	co: &CoId,
	recipient: &Did,
) -> Result<Vec<Task>, anyhow::Error> {
	let matching = head_task_selector(co, recipient);
	network_queue_tasks(context.clone(), list_name.to_owned(), move |task| task.tags.matches(&matching))
		.try_collect()
		.await
}

#[cfg(test)]
pub(crate) async fn read_intent(context: &CoContext, task: &Task) -> Result<HeadsDeliveryIntent, anyhow::Error> {
	let payload = task.payload.ok_or_else(|| anyhow::anyhow!("head task has no payload"))?;
	let local_co = context.local_co_reducer().await?;
	Ok(local_co.storage().get_deserialized(&payload).await?)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		library::{
			head_delivery::HeadsDeliveryIntent,
			network_queue::{
				network_queue_action, network_queue_task_complete, network_queue_task_doing,
				network_queue_task_in_list, network_queue_tasks, TaskState, LIST_NAME_BACKLOG, LIST_NAME_DOING,
			},
		},
		state::{query_core, QueryExt},
		Application, ApplicationBuilder, CoContext,
	};
	use co_core_board::{BoardAction, Task, TaskId};
	use co_identity::PrivateIdentity;
	use co_primitives::{tags, CoConnectivity, CoId, Did};

	async fn queue_test_context(name: &str) -> (Application, CoContext) {
		let application = ApplicationBuilder::new_memory(format!("head-queue-{name}"))
			.without_keychain()
			.build()
			.await
			.unwrap();
		let context = application.context().clone();
		(application, context)
	}

	fn test_intent(recipient: &str) -> HeadsDeliveryIntent {
		HeadsDeliveryIntent {
			co: CoId::from("co-test"),
			from: Did::from("did:key:sender"),
			recipient: Did::from(recipient),
			connectivity: CoConnectivity::default(),
		}
	}

	async fn task_record(context: &CoContext, task_id: &str) -> Option<Task> {
		let local_co = context.local_co_reducer().await.unwrap();
		let (_, task) = query_core(CO_CORE_NAME_NETWORK_QUEUE)
			.with_default()
			.map(|board| board.tasks)
			.get_value(task_id.to_owned())
			.execute_reducer(&local_co)
			.await
			.unwrap();
		task
	}

	async fn all_tasks(context: &CoContext, list_name: &str) -> Vec<Task> {
		network_queue_tasks(context.clone(), list_name.to_owned(), |_| true)
			.try_collect()
			.await
			.unwrap()
	}

	async fn seed_intent_task(
		context: &CoContext,
		list: &str,
		id: impl Into<TaskId>,
		intent: &HeadsDeliveryIntent,
	) -> Task {
		let local_co = context.local_co_reducer().await.unwrap();
		let identity = context.local_identity();
		let (_, co) = local_co.co().await.unwrap();
		ensure_network_queue_core(context.cores(), &local_co, &identity, co)
			.await
			.unwrap();
		let task = Task {
			id: id.into(),
			name: task_name(intent),
			tags: task_tags(intent),
			payload: Some(local_co.storage().set_serialized(intent).await.unwrap()),
			lock: None,
		};
		local_co
			.push(
				&identity,
				CO_CORE_NAME_NETWORK_QUEUE,
				&BoardAction::TaskCreate { list: list.to_owned(), task: task.clone(), after: None },
			)
			.await
			.unwrap();
		task
	}

	async fn seed_task(context: &CoContext, list: &str, task: Task) {
		let local_co = context.local_co_reducer().await.unwrap();
		let identity = context.local_identity();
		let (_, co) = local_co.co().await.unwrap();
		ensure_network_queue_core(context.cores(), &local_co, &identity, co)
			.await
			.unwrap();
		local_co
			.push(
				&identity,
				CO_CORE_NAME_NETWORK_QUEUE,
				&BoardAction::TaskCreate { list: list.to_owned(), task, after: None },
			)
			.await
			.unwrap();
	}

	#[tokio::test]
	async fn repeated_enqueue_replaces_with_fresh_id_and_newest_payload() {
		let (application, context) = queue_test_context("replace").await;
		let mut intent = test_intent("did:key:one");
		let first_id = enqueue_latest(&context, intent.clone()).await.unwrap();
		intent.from = Did::from("did:key:sender-2");
		let second_id = enqueue_latest(&context, intent.clone()).await.unwrap();
		intent.from = Did::from("did:key:sender-3");
		let third_id = enqueue_latest(&context, intent.clone()).await.unwrap();

		assert_ne!(first_id, second_id);
		assert_ne!(second_id, third_id);
		assert_ne!(first_id, third_id);

		let tasks = head_tasks(&context, LIST_NAME_BACKLOG, &intent.co, &intent.recipient)
			.await
			.unwrap();
		assert_eq!(tasks.iter().map(|task| &task.id).collect::<Vec<_>>(), [&third_id]);
		assert_eq!(read_intent(&context, &tasks[0]).await.unwrap(), intent);
		assert!(task_record(&context, &first_id).await.is_none());
		assert!(task_record(&context, &second_id).await.is_none());
		application.shutdown_application().await;
	}

	#[tokio::test]
	async fn stale_snapshot_cannot_claim_replaced_task_and_fresh_task_executes() {
		let (application, context) = queue_test_context("stale-snapshot").await;
		let intent = test_intent("did:key:one");
		let stale_id = enqueue_latest(&context, intent.clone()).await.unwrap();
		let local_co = context.local_co_reducer().await.unwrap();
		let identity = context.local_identity().boxed();
		let stale_snapshot = network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, &stale_id)
			.await
			.unwrap()
			.expect("stale task starts in backlog");

		let fresh_id = enqueue_latest(&context, intent).await.unwrap();
		assert_ne!(stale_id, fresh_id);
		assert!(network_queue_task_doing(&identity, &local_co, &stale_snapshot, "stale-worker")
			.await
			.unwrap()
			.is_none());
		assert!(network_queue_task_in_list(&local_co, LIST_NAME_DOING, &stale_id)
			.await
			.unwrap()
			.is_none());

		let fresh_snapshot = network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, &fresh_id)
			.await
			.unwrap()
			.expect("fresh task remains in backlog");
		let fresh = network_queue_task_doing(&identity, &local_co, &fresh_snapshot, "fresh-worker")
			.await
			.unwrap()
			.expect("fresh task wins the claim");
		let (action, _) = network_queue_action(&local_co, &fresh, "fresh-worker").await.unwrap();
		assert!(matches!(
			action,
			crate::Action::NetworkTaskExecute { task_id, .. } if task_id == fresh_id
		));
		application.shutdown_application().await;
	}

	#[tokio::test]
	async fn enqueue_while_doing_keeps_claim_and_replaces_successor_with_fresh_id() {
		let (application, context) = queue_test_context("successor").await;
		let intent = test_intent("did:key:one");
		let claimed_id = enqueue_latest(&context, intent.clone()).await.unwrap();
		let task = head_tasks(&context, LIST_NAME_BACKLOG, &intent.co, &intent.recipient)
			.await
			.unwrap()
			.remove(0);
		let local_co = context.local_co_reducer().await.unwrap();
		let identity = context.local_identity().boxed();
		let claimed = network_queue_task_doing(&identity, &local_co, &task, "test-lock")
			.await
			.unwrap()
			.expect("claim succeeds");
		assert_eq!(claimed.id, claimed_id);
		assert_eq!(claimed.lock.as_deref(), Some("test-lock"));

		let mut newer = intent.clone();
		newer.from = Did::from("did:key:sender-2");
		let first_successor_id = enqueue_latest(&context, newer.clone()).await.unwrap();
		newer.from = Did::from("did:key:sender-3");
		let latest_successor_id = enqueue_latest(&context, newer.clone()).await.unwrap();
		assert_ne!(claimed_id, first_successor_id);
		assert_ne!(first_successor_id, latest_successor_id);
		assert_ne!(claimed_id, latest_successor_id);

		let doing = head_tasks(&context, LIST_NAME_DOING, &intent.co, &intent.recipient)
			.await
			.unwrap();
		assert_eq!(doing, [claimed]);
		let backlog = head_tasks(&context, LIST_NAME_BACKLOG, &intent.co, &intent.recipient)
			.await
			.unwrap();
		assert_eq!(backlog.iter().map(|task| &task.id).collect::<Vec<_>>(), [&latest_successor_id]);
		assert_eq!(read_intent(&context, &backlog[0]).await.unwrap(), newer);
		assert!(task_record(&context, &first_successor_id).await.is_none());
		application.shutdown_application().await;
	}

	#[tokio::test]
	async fn enqueue_collapses_preexisting_duplicates_into_new_task_record() {
		let (application, context) = queue_test_context("duplicates").await;
		let intent = test_intent("did:key:one");
		seed_intent_task(&context, LIST_NAME_BACKLOG, "duplicate-a", &intent).await;
		seed_intent_task(&context, LIST_NAME_BACKLOG, "duplicate-b", &intent).await;

		let fresh_id = enqueue_latest(&context, intent.clone()).await.unwrap();

		assert_ne!(fresh_id, "duplicate-a");
		assert_ne!(fresh_id, "duplicate-b");
		let backlog = head_tasks(&context, LIST_NAME_BACKLOG, &intent.co, &intent.recipient)
			.await
			.unwrap();
		assert_eq!(backlog.iter().map(|task| &task.id).collect::<Vec<_>>(), [&fresh_id]);
		assert!(task_record(&context, "duplicate-a").await.is_none());
		assert!(task_record(&context, "duplicate-b").await.is_none());
		application.shutdown_application().await;
	}

	#[tokio::test]
	async fn enqueue_does_not_replace_unrelated_recipient_or_task_type() {
		let (application, context) = queue_test_context("unrelated").await;
		let target = test_intent("did:key:target");
		let other_recipient_intent = test_intent("did:key:other");
		let other_recipient =
			seed_intent_task(&context, LIST_NAME_BACKLOG, "other-recipient", &other_recipient_intent).await;
		let other_type = Task {
			id: "other-type".to_owned(),
			name: "Unrelated task type".to_owned(),
			tags: tags!(
				"co": target.co.to_string(),
				"task-type": "other-task-type",
				TAG_RECIPIENT: target.recipient.clone(),
			),
			payload: None,
			lock: None,
		};
		seed_task(&context, LIST_NAME_BACKLOG, other_type.clone()).await;

		let fresh_id = enqueue_latest(&context, target.clone()).await.unwrap();

		let backlog = all_tasks(&context, LIST_NAME_BACKLOG).await;
		assert_eq!(backlog.len(), 3);
		assert_eq!(backlog.iter().find(|task| task.id == other_recipient.id), Some(&other_recipient));
		assert_eq!(backlog.iter().find(|task| task.id == other_type.id), Some(&other_type));
		assert_eq!(
			head_tasks(&context, LIST_NAME_BACKLOG, &target.co, &target.recipient)
				.await
				.unwrap()
				.iter()
				.map(|task| &task.id)
				.collect::<Vec<_>>(),
			[&fresh_id]
		);
		application.shutdown_application().await;
	}

	async fn assert_completion_enqueue_order(name: &str, completion_first: bool) {
		let (application, context) = queue_test_context(name).await;
		let mut intent = test_intent("did:key:one");
		let old_id = enqueue_latest(&context, intent.clone()).await.unwrap();
		let local_co = context.local_co_reducer().await.unwrap();
		let identity = context.local_identity().boxed();
		let snapshot = network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, &old_id)
			.await
			.unwrap()
			.unwrap();
		let claimed = network_queue_task_doing(&identity, &local_co, &snapshot, "test-lock")
			.await
			.unwrap()
			.unwrap();
		intent.from = Did::from("did:key:fresh-sender");

		let fresh_id = if completion_first {
			network_queue_task_complete(
				&identity,
				&local_co,
				&claimed,
				"test-lock",
				TaskState::Backlog,
				head_task_selector_from_task(&claimed),
			)
			.await
			.unwrap();
			enqueue_latest(&context, intent.clone()).await.unwrap()
		} else {
			let fresh_id = enqueue_latest(&context, intent.clone()).await.unwrap();
			network_queue_task_complete(
				&identity,
				&local_co,
				&claimed,
				"test-lock",
				TaskState::Backlog,
				head_task_selector_from_task(&claimed),
			)
			.await
			.unwrap();
			fresh_id
		};

		assert_ne!(old_id, fresh_id);
		assert!(head_tasks(&context, LIST_NAME_DOING, &intent.co, &intent.recipient)
			.await
			.unwrap()
			.is_empty());
		let backlog = head_tasks(&context, LIST_NAME_BACKLOG, &intent.co, &intent.recipient)
			.await
			.unwrap();
		assert_eq!(backlog.iter().map(|task| &task.id).collect::<Vec<_>>(), [&fresh_id]);
		assert_eq!(read_intent(&context, &backlog[0]).await.unwrap(), intent);
		assert!(task_record(&context, &old_id).await.is_none());
		application.shutdown_application().await;
	}

	#[tokio::test]
	async fn completion_and_enqueue_converge_to_only_the_fresh_successor() {
		assert_completion_enqueue_order("completion-first", true).await;
		assert_completion_enqueue_order("enqueue-first", false).await;
	}

	#[test]
	fn selector_from_task_requires_a_complete_head_delivery_key() {
		let intent = test_intent("did:key:one");
		let matching = Task {
			id: "matching".to_owned(),
			name: task_name(&intent),
			tags: task_tags(&intent),
			payload: None,
			lock: None,
		};
		assert_eq!(head_task_selector_from_task(&matching), Some(head_task_selector(&intent.co, &intent.recipient)));

		let mut wrong_type = matching.clone();
		wrong_type.tags = tags!(
			"co": intent.co.to_string(),
			"task-type": "other",
			TAG_RECIPIENT: intent.recipient.clone(),
		);
		assert!(head_task_selector_from_task(&wrong_type).is_none());

		let mut missing_recipient = matching;
		missing_recipient.tags = tags!("co": intent.co.to_string(), "task-type": HEADS_DELIVERY_TASK_TYPE);
		assert!(head_task_selector_from_task(&missing_recipient).is_none());
	}
}
