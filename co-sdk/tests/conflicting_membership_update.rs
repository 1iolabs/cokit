use co_core_co::CoAction;
use co_core_file::{File, FileAction, FolderNode, Node};
use co_core_membership::Memberships;
use co_sdk::{
	state::{self, query_core, QueryExt},
	tags, AbsolutePath, ApplicationBuilder, CoId, CoReducer, CoReducerFactory, Cores, CreateCo, DidKeyIdentity,
	DidKeyProvider, Identity, CO_CORE_FILE, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use co_storage::TmpDir;
use futures::{join, StreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	future::ready,
	time::Duration,
};
use tokio::time::timeout;

/// See:
/// - https://gitlab.1io.com/1io/co-sdk/-/issues/59
#[tokio::test]
async fn test_conflicting_membership_update() {
	let timeout_duration = Duration::from_secs(5);
	let tmp = TmpDir::new("co");

	// application
	let application = ApplicationBuilder::new_with_path("test".to_owned(), tmp.path().to_owned())
		.without_keychain()
		.with_setting("co-local-watch", false)
		.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
		.build()
		.await
		.expect("application");

	// create identity
	let identity = DidKeyIdentity::generate(None);
	let local_co = application.local_co_reducer().await.unwrap();
	let provider = DidKeyProvider::new(local_co.clone(), CO_CORE_NAME_KEYSTORE);
	provider.store(&identity, None).await.unwrap();

	// create co
	let co = application
		.create_co(
			identity.clone(),
			CreateCo { algorithm: Some(Default::default()), id: "co".into(), name: "co".into() },
		)
		.await
		.unwrap();
	co.push(
		&identity,
		CO_CORE_NAME_CO,
		&CoAction::CoreCreate {
			core: "file".to_owned(),
			binary: Cores::default().binary(CO_CORE_FILE).expect(CO_CORE_FILE),
			tags: tags!( "core": CO_CORE_FILE ),
		},
	)
	.await
	.unwrap();

	// application instance two
	let application2 = ApplicationBuilder::new_with_path("test2".to_owned(), tmp.path().to_owned())
		.without_keychain()
		.with_setting("co-local-watch", false)
		.build()
		.await
		.expect("application2");
	let local_co2 = application2.local_co_reducer().await.unwrap();
	let co2 = application2.co().try_co_reducer(&CoId::new("co")).await.unwrap();

	// write to both
	let co_state = co
		.push(
			&identity,
			"file",
			&FileAction::Create {
				path: "/".try_into().unwrap(),
				node: Node::Folder(FolderNode {
					name: "folder".to_owned(),
					create_time: 0,
					modify_time: 0,
					tags: tags!(),
					owner: identity.identity().to_owned(),
					mode: 0o665,
				}),
				recursive: false,
			},
		)
		.await
		.unwrap();
	let co2_state = co2
		.push(
			&identity,
			"file",
			&FileAction::Create {
				path: "/".try_into().unwrap(),
				node: Node::Folder(FolderNode {
					name: "folder2".to_owned(),
					create_time: 0,
					modify_time: 0,
					tags: tags!(),
					owner: identity.identity().to_owned(),
					mode: 0o665,
				}),
				recursive: false,
			},
		)
		.await
		.unwrap();

	tracing::info!("co1 count {:?}", count_folders(&co).await);
	tracing::info!("co2 count {:?}", count_folders(&co2).await);
	tracing::info!(state1 = ?co.reducer_state().await, state2 = ?co2.reducer_state().await, "conflict");
	// let (_, m) = query_core::<Memberships>(CO_CORE_NAME_MEMBERSHIP)
	// 	.execute_reducer(&local_co)
	// 	.await
	// 	.unwrap();
	// let (_, m2) = query_core::<Memberships>(CO_CORE_NAME_MEMBERSHIP)
	// 	.execute_reducer(&local_co2)
	// 	.await
	// 	.unwrap();
	// tracing::info!("m1: {:?}", m.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);
	// tracing::info!("m2: {:?}", m2.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);
	// application.co().refresh(local_co.clone()).await.unwrap();
	// application2.co().refresh(local_co2.clone()).await.unwrap();
	// let (_, m) = query_core::<Memberships>(CO_CORE_NAME_MEMBERSHIP)
	// 	.execute_reducer(&local_co)
	// 	.await
	// 	.unwrap();
	// let (_, m2) = query_core::<Memberships>(CO_CORE_NAME_MEMBERSHIP)
	// 	.execute_reducer(&local_co2)
	// 	.await
	// 	.unwrap();
	// tracing::info!("u1: {:?}", m.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);
	// tracing::info!("u2: {:?}", m2.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);
	// tracing::info!("co1 next count {:?}", count_folders(&co).await);
	// tracing::info!("co2 next count {:?}", count_folders(&co2).await);

	async fn count_folders(co: &CoReducer) -> usize {
		let (storage, files) = query_core::<File>("file").execute_reducer(co).await.unwrap();
		let nodes = state::into_collection::<BTreeMap<_, _>, _, _, _>(&storage, &files.nodes)
			.await
			.unwrap();
		let nodes_root_dag = nodes.get(AbsolutePath::new_unchecked("/")).unwrap();
		let nodes_root: BTreeSet<Node> = state::into_collection(&storage, nodes_root_dag).await.unwrap();
		nodes_root.len()
	}

	// check: refresh and wait until state changed
	let check1 = async {
		application.co().refresh(local_co.clone()).await.unwrap();
		timeout(
			timeout_duration,
			co.reducer_state_stream()
				.filter(|state| ready(state != &co_state))
				.boxed()
				.next(),
		)
		.await
		.unwrap();
		assert_eq!(count_folders(&co).await, 2);
	};

	// check2: refresh and wait until state changed
	let check2 = async {
		application2.co().refresh(local_co2.clone()).await.unwrap();
		timeout(
			timeout_duration,
			co2.reducer_state_stream()
				.filter(|state| ready(state != &co2_state))
				.boxed()
				.next(),
		)
		.await
		.unwrap();
		assert_eq!(count_folders(&co2).await, 2);
	};
	join!(check1, check2);

	// write more data and check we only got one CoState with one head left
	co.push(
		&identity,
		"file",
		&FileAction::Create {
			path: "/".try_into().unwrap(),
			node: Node::Folder(FolderNode {
				name: "folder3".to_owned(),
				create_time: 0,
				modify_time: 0,
				tags: tags!(),
				owner: identity.identity().to_owned(),
				mode: 0o665,
			}),
			recursive: false,
		},
	)
	.await
	.unwrap();
	let (_, memberships) = query_core::<Memberships>(CO_CORE_NAME_MEMBERSHIP)
		.execute_reducer(&local_co)
		.await
		.unwrap();
	//println!("memberships: {:?}", memberships.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);
	assert_eq!(
		memberships
			.memberships
			.iter()
			.find(|i| i.id.as_str() == "co")
			.unwrap()
			.state
			.len(),
		1
	);
	assert_eq!(
		memberships
			.memberships
			.iter()
			.find(|i| i.id.as_str() == "co")
			.unwrap()
			.state
			.first()
			.unwrap()
			.heads
			.len(),
		1
	);
	// println!("u1: {:?}", m.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);
	// println!("u2: {:?}", m2.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);
}
