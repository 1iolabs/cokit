use co_core_co::CoAction;
use co_core_file::{FileAction, FolderNode};
use co_primitives::AbsolutePathOwned;
use co_sdk::{tags, Cores, Identity, CO_CORE_NAME_CO};
use futures::StreamExt;
use helper::{instance::Instances, shared_co::SharedCo};
use std::{
	future::ready,
	time::{Duration, SystemTime},
};
use tokio::time::timeout;
use tokio_stream::wrappers::WatchStream;

pub mod helper;

/// Push changes to peer.
#[tokio::test]
async fn test_push() {
	let mut instances = Instances::new("test_push");
	let shared_co = SharedCo::create(&mut instances, "shared").await;

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
	let (peer0_state, peer0_heads) = peer0.reducer_state().await;
	let peer0_state = peer0_state.unwrap();

	// peer1: wait for state/heads to be updated
	let (peer1, _identity1) = shared_co.reducer(1, "shared").await;
	let mut peer1_state_future = WatchStream::new(peer1.watch().await)
		.filter_map(ready)
		.filter(|(state, _heads)| ready(state == &peer0_state))
		.take(1);
	let (peer1_state, peer1_heads) = timeout(Duration::from_secs(1), peer1_state_future.next())
		.await
		.expect("to sync in time")
		.expect("state");
	assert_eq!(peer1_state, peer0_state);
	assert_eq!(peer1_heads, peer0_heads);

	// peer0: create file
	let folder = FolderNode {
		name: "test".to_owned(),
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
	let (peer0_state, peer0_heads) = peer0.reducer_state().await;
	let peer0_state = peer0_state.unwrap();

	// peer1: wait for state/heads to be updated
	let (peer1, _identity1) = shared_co.reducer(1, "shared").await;
	let mut peer1_state_future = WatchStream::new(peer1.watch().await)
		.filter_map(ready)
		.filter(|(state, _heads)| ready(state == &peer0_state))
		.take(1);
	let (peer1_state, peer1_heads) = timeout(Duration::from_secs(1), peer1_state_future.next())
		.await
		.expect("to sync in time")
		.expect("state");
	assert_eq!(peer1_state, peer0_state);
	assert_eq!(peer1_heads, peer0_heads);
}
