use co_core_co::CoAction;
use co_core_file::{File, FileAction, FolderNode, Node};
use co_core_membership::Memberships;
use co_sdk::{
	state, tags, AbsolutePath, ApplicationBuilder, CoId, CoReducer, CoReducerFactory, Cores, CreateCo, DidKeyIdentity,
	DidKeyProvider, Identity, TmpDir, CO_CORE_FILE, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use std::collections::{BTreeMap, BTreeSet};

/// See:
/// - https://gitlab.1io.com/1io/co-sdk/-/issues/59
#[tokio::test]
async fn test_conflicting_membership_update() {
	let tmp = TmpDir::new("co");

	// application
	let application = ApplicationBuilder::new_with_path("test".to_owned(), tmp.path().to_owned())
		.without_keychain()
		.with_setting("co-local-watch", false)
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
	co.push(
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
	co2.push(
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
	// println!("co1: {:?} / {:?}", co.co_state().await, co.heads().await);
	// println!("co2: {:?} / {:?}", co2.co_state().await, co2.heads().await);
	// let m: Memberships = local_co.state(CO_CORE_NAME_MEMBERSHIP).await.unwrap();
	// let m2: Memberships = local_co2.state(CO_CORE_NAME_MEMBERSHIP).await.unwrap();
	// println!("m1: {:?}", m.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);
	// println!("m2: {:?}", m2.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);

	// // refresh
	// local_co.refresh(local_co.clone()).await.unwrap();
	// // local_co2.refresh(local_co2.clone()).await.unwrap();

	// let m: Memberships = local_co.state(CO_CORE_NAME_MEMBERSHIP).await.unwrap();
	// let m2: Memberships = local_co2.state(CO_CORE_NAME_MEMBERSHIP).await.unwrap();
	// println!("u1: {:?}", m.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);
	// println!("u2: {:?}", m2.memberships.iter().find(|i| i.id.as_str() == "co").unwrap().state);

	async fn test_folders_exists(co: &CoReducer) {
		let files: File = co.state("file").await.unwrap();
		let nodes = state::into_collection::<BTreeMap<_, _>, _, _, _>(&co.storage(), &files.nodes)
			.await
			.unwrap();
		let nodes_root_dag = nodes.get(AbsolutePath::new_unchecked("/")).unwrap();
		let nodes_root: BTreeSet<Node> = state::into_collection(&co.storage(), nodes_root_dag).await.unwrap();
		assert_eq!(nodes_root.len(), 2);
	}

	// check
	//  note: force update the co instance too and wait for update because of
	// [`co_sdk::reducer::core_resolver::membership::MembershipCoreResolver`]
	local_co.refresh(local_co.clone()).await.unwrap();
	co.refresh(local_co.clone()).await.unwrap();
	test_folders_exists(&co).await;

	// check2
	local_co2.refresh(local_co2.clone()).await.unwrap();
	co2.refresh(local_co2.clone()).await.unwrap();
	test_folders_exists(&co2).await;

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
	let memberships: Memberships = local_co.state(CO_CORE_NAME_MEMBERSHIP).await.unwrap();
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
