// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_core_co::CoAction;
use co_core_file::{FileAction, FolderNode};
use co_network::connections::{ConnectionAction, ConnectionMessage, ReleaseAction};
use co_primitives::{AbsolutePathOwned, TagsAction};
use co_sdk::{
	tags, Action, Application, CoDate, CoId, Cores, CreateCo, Did, DidKeyIdentity, HeadsDeliveryCompleteAction,
	HeadsDeliveryOutcome, HeadsDeliveryPhase, Identity, CO_CORE_NAME_CO,
};
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use helper::{instance::Instances, shared_co::SharedCo};
use std::{future::ready, pin::Pin, time::Duration};
use tokio::time::{sleep, timeout};

pub mod helper;

fn queued_completions(
	application: &Application,
	co: CoId,
	recipient: Did,
	phase: HeadsDeliveryPhase,
) -> Pin<Box<dyn Stream<Item = HeadsDeliveryCompleteAction> + Send>> {
	Box::pin(application.actions().filter_map(move |action| {
		let co = co.clone();
		let recipient = recipient.clone();
		async move {
			match action {
				Action::HeadsDeliveryComplete(done)
					if done.co == co
						&& done.recipient == recipient
						&& done.phase == phase
						&& matches!(&done.outcome, HeadsDeliveryOutcome::Queued) =>
				{
					Some(done)
				},
				_ => None,
			}
		}
	}))
}

async fn wait_for_queued(
	completions: &mut Pin<Box<dyn Stream<Item = HeadsDeliveryCompleteAction> + Send>>,
) -> HeadsDeliveryCompleteAction {
	timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap()
}

async fn head_backlog_count(application: &Application, co: &CoId, recipient: &Did) -> usize {
	let local_co = application.context().local_co_reducer().await.unwrap();
	let tasks = co_sdk::state::board::tasks(
		local_co.storage(),
		local_co.reducer_state().await,
		"network_queue".to_owned(),
		"backlog".to_owned(),
	);
	tasks
		.try_filter(|task| {
			ready(
				task.tags.string("task-type") == Some("co-heads-did")
					&& task.tags.string("co") == Some(co.as_str())
					&& task.tags.string("recipient") == Some(recipient.as_str()),
			)
		})
		.try_collect::<Vec<_>>()
		.await
		.unwrap()
		.len()
}

async fn wait_for_head_backlog_count(application: &Application, co: &CoId, recipient: &Did, expected: usize) {
	timeout(Duration::from_secs(10), async {
		loop {
			if head_backlog_count(application, co, recipient).await == expected {
				break;
			}
			sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.unwrap();
}

#[tokio::test]
async fn active_offline_participant_uses_targeted_queue() {
	let mut instances = Instances::new("participant-head-queue");
	let peer = instances.create().await;
	let identity = peer.create_identity().await;
	let offline = DidKeyIdentity::generate(None);
	let co = CoId::from("offline-participant");
	let recipient = offline.identity().to_owned();
	let reducer = peer
		.application
		.create_co(identity.clone(), CreateCo::new(co.clone(), None))
		.await
		.unwrap();
	reducer
		.push(
			&identity,
			CO_CORE_NAME_CO,
			&CoAction::ParticipantInvite { participant: offline.identity().to_owned(), tags: Default::default() },
		)
		.await
		.unwrap();
	let mut join_admission =
		queued_completions(&peer.application, co.clone(), recipient.clone(), HeadsDeliveryPhase::Admission);
	let mut join_execution =
		queued_completions(&peer.application, co.clone(), recipient.clone(), HeadsDeliveryPhase::Execution);
	reducer
		.push(
			&identity,
			CO_CORE_NAME_CO,
			&CoAction::ParticipantJoin { participant: offline.identity().to_owned(), tags: Default::default() },
		)
		.await
		.unwrap();
	wait_for_queued(&mut join_admission).await;
	wait_for_queued(&mut join_execution).await;
	wait_for_head_backlog_count(&peer.application, &co, &recipient, 1).await;

	let mut tag_admission =
		queued_completions(&peer.application, co.clone(), recipient.clone(), HeadsDeliveryPhase::Admission);
	let mut tag_execution =
		queued_completions(&peer.application, co.clone(), recipient.clone(), HeadsDeliveryPhase::Execution);
	reducer
		.push(&identity, CO_CORE_NAME_CO, &CoAction::Tags { action: TagsAction::insert(tags!("change": "queued")) })
		.await
		.unwrap();
	wait_for_queued(&mut tag_admission).await;
	wait_for_queued(&mut tag_execution).await;
	wait_for_head_backlog_count(&peer.application, &co, &recipient, 1).await;
}

/// Push changes to peer.
#[tokio::test]
async fn test_push() {
	let timeout_duration = Duration::from_secs(15);
	let mut instances = Instances::new("test_push");
	let shared_co = SharedCo::create(&mut instances, "shared").await;

	// disconnect
	let context0 = shared_co.peers.first().unwrap().0.application.co();
	context0
		.network_connections()
		.await
		.unwrap()
		.dispatch(ConnectionMessage::Action(ConnectionAction::Release(ReleaseAction { id: "shared".into() })))
		.unwrap();

	// peer0: create a core
	let (peer0, identity0) = shared_co.reducer(0, "shared").await;
	peer0
		.push(
			&identity0,
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate {
				core: "file".to_owned(),
				binary: Cores::default().binary("co-core-file").unwrap(),
				tags: Default::default(),
			},
		)
		.await
		.unwrap();
	let peer0_state = peer0.reducer_state().await;

	// peer1: wait for state/heads to be updated
	let (peer1, _identity1) = shared_co.reducer(1, "shared").await;
	let peer1_state_future = peer1
		.reducer_state_stream()
		.filter(|state| ready(state == &peer0_state))
		.take(1);
	pin_mut!(peer1_state_future);
	let peer1_state = timeout(timeout_duration, peer1_state_future.next())
		.await
		.expect("to sync in time")
		.expect("state");
	assert_eq!(peer1_state, peer0_state);

	// peer0: create folders
	for i in 0..3 {
		let folder = FolderNode {
			name: format!("test-{}", i),
			create_time: context0.date().now(),
			modify_time: context0.date().now(),
			tags: tags!(),
			owner: identity0.identity().to_owned(),
			mode: 0o665,
		};
		peer0
			.push(
				&identity0,
				"file",
				&FileAction::Create {
					path: AbsolutePathOwned::new("/".to_owned()).unwrap(),
					node: co_core_file::Node::Folder(folder),
					recursive: false,
				},
			)
			.await
			.unwrap();
		sleep(Duration::from_millis(i)).await;
	}
	let peer0_state = peer0.reducer_state().await;

	// peer1: wait for state/heads to be updated
	let (peer1, _identity1) = shared_co.reducer(1, "shared").await;
	let peer1_state_future = peer1
		.reducer_state_stream()
		.filter(|state| ready(state == &peer0_state))
		.take(1);
	pin_mut!(peer1_state_future);
	let peer1_state = timeout(timeout_duration, peer1_state_future.next())
		.await
		.expect("to sync in time")
		.expect("state");
	assert_eq!(peer1_state, peer0_state);
}
