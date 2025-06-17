use co_core_co::CoAction;
use co_core_file::{FileAction, FolderNode};
use co_primitives::AbsolutePathOwned;
use co_sdk::{tags, ConnectionAction, ConnectionMessage, Cores, Identity, ReleaseAction, CO_CORE_NAME_CO};
use futures::{pin_mut, StreamExt};
use helper::{instance::Instances, shared_co::SharedCo};
use std::{
	future::ready,
	time::{Duration, SystemTime},
};
use tokio::time::{sleep, timeout};

pub mod helper;

/// Push changes to peer.
#[tokio::test]
async fn test_push() {
	let timeout_duration = Duration::from_secs(15);
	let mut instances = Instances::new("test_push");
	let shared_co = SharedCo::create(&mut instances, "shared").await;

	// disconnect
	let context0 = shared_co.peers.get(0).unwrap().0.application.co();
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
			create_time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis(),
			modify_time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis(),
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
