use cid::Cid;
use co_core_co::CoAction;
use co_core_file::{File, FileAction, FolderNode, Node};
use co_core_membership::Memberships;
use co_sdk::{
	ipld_resolve_recursive,
	state::{self, query_core, QueryExt},
	tags, AbsolutePath, ApplicationBuilder, BlockStorageExt, CoContext, CoId, CoReducer, CoReducerFactory,
	CoReducerState, CoStorage, Cores, CreateCo, DidKeyIdentity, DidKeyProvider, Identity, MonotonicCoDate,
	MonotonicCoUuid, CO_CORE_FILE, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use co_storage::TmpDir;
use futures::{join, pin_mut, stream, StreamExt, TryStreamExt};
use ipld_core::ipld::Ipld;
use std::{
	collections::{BTreeMap, BTreeSet},
	future::ready,
	time::Duration,
};
use tokio::time::timeout;

async fn trace_state(co: &str, storage: &CoStorage, reducer_state: &CoReducerState) {
	tracing::trace!(
		"state: {}: {:#?}",
		co,
		ipld_resolve_recursive(storage, Ipld::Link(reducer_state.state().unwrap()), true)
			.await
			.unwrap()
	);
}
async fn trace_heads(co: &str, context: &CoContext, storage: &CoStorage, reducer_state: &CoReducerState) {
	// fn pretty_print_with_indent<T: std::fmt::Debug>(value: &T, indent: usize) {
	// 	let formatted = format!("{:#?}", value);
	// 	let indent_str = " ".repeat(indent);
	// 	let indented = formatted
	// 		.lines()
	// 		.map(|line| format!("{}{}", indent_str, line))
	// 		.collect::<Vec<_>>()
	// 		.join("\n");
	// 	println!("{}", indented);
	// }
	tracing::trace!("heads: {}: {:?}", co, reducer_state.heads());
	let entries = context
		.entries_from_heads(CoId::from(co), storage.clone(), reducer_state.heads().clone())
		.await
		.unwrap()
		.enumerate()
		.map(|(index, result)| result.map(|ok| (index, ok)));
	pin_mut!(entries);
	while let Some((index, entry)) = entries.try_next().await.unwrap() {
		tracing::trace!("{} (#{})", entry.cid(), index);
		tracing::trace!(
			"    {:?}",
			ipld_resolve_recursive(storage, Ipld::Link(entry.entry().payload), true)
				.await
				.unwrap()
		);
	}
}

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
		.with_co_date(MonotonicCoDate::default())
		.with_co_uuid(MonotonicCoUuid::default())
		.build()
		.await
		.expect("application");

	// create identity
	let identity = DidKeyIdentity::generate(Some(&vec![1; 32]));
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

	// log
	let local_state = local_co.reducer_state().await;
	let co_state = co.reducer_state().await;
	trace_state("local", &local_co.storage(), &local_state).await;
	trace_heads("local", application.co(), &local_co.storage(), &local_state).await;
	trace_state("shared", &co.storage(), &co_state).await;
	trace_heads("shared", application.co(), &co.storage(), &co_state).await;

	// log heads

	// println!(
	// 	"local-heads: {:#?}",
	// 	ipld_resolve_recursive(
	// 		&local_co.storage(),
	// 		Ipld::List(local_state.heads().into_iter().map(Ipld::Link).collect()),
	// 		true
	// 	)
	// 	.await
	// 	.unwrap()
	// );

	// application instance two
	let application2 = ApplicationBuilder::new_with_path("test2".to_owned(), tmp.path().to_owned())
		.without_keychain()
		.with_setting("co-local-watch", false)
		.with_co_date(MonotonicCoDate::default())
		.build()
		.await
		.expect("application2");
	let local_co2 = application2.local_co_reducer().await.unwrap();
	let co2 = application2.co().try_co_reducer(&CoId::new("co")).await.unwrap();

	// validate
	tracing::info!(
		co = ?co.reducer_state().await,
		co2 = ?co2.reducer_state().await,
		local_co = ?local_co.reducer_state().await,
		local_co2 = ?local_co2.reducer_state().await,
		"test-start"
	);
	assert_eq!(co.reducer_state().await, co2.reducer_state().await);
	assert_eq!(local_co.reducer_state().await, local_co2.reducer_state().await);

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
	tracing::info!(co = ?co.reducer_state().await, co2 = ?co2.reducer_state().await, "test-conflict");
	tracing::info!("local-conflict");
	trace_heads("local", application.co(), &local_co.storage(), &local_co.reducer_state().await).await;
	tracing::info!("local2-conflict");
	trace_heads("local", application.co(), &local_co2.storage(), &local_co2.reducer_state().await).await;

	// tracing::info!("co1 count {:?}", count_folders(&co).await);
	// tracing::info!("co2 count {:?}", count_folders(&co2).await);
	// tracing::info!(state1 = ?co.reducer_state().await, state2 = ?co2.reducer_state().await, "conflict");
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

	// refresh
	tracing::info!("local-refresh");
	application.co().refresh(local_co.clone()).await.unwrap();
	trace_state("local", &local_co.storage(), &local_co.reducer_state().await).await;
	trace_heads("local", application.co(), &local_co.storage(), &local_co.reducer_state().await).await;
	tracing::info!("local2-refresh");
	application2.co().refresh(local_co2.clone()).await.unwrap();
	trace_state("local", &local_co2.storage(), &local_co2.reducer_state().await).await;
	trace_heads("local", application2.co(), &local_co2.storage(), &local_co2.reducer_state().await).await;

	// check: refresh and wait until state changed
	tracing::info!(co = ?co.reducer_state().await, co2 = ?co2.reducer_state().await, "test-refresh");
	let check1 = async {
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
	tracing::info!(co = ?co.reducer_state().await, co2 = ?co2.reducer_state().await, "test-join");

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
	let (storage, memberships) = query_core::<Memberships>(CO_CORE_NAME_MEMBERSHIP)
		.execute_reducer(&local_co)
		.await
		.unwrap();
	let heads = stream::iter(
		memberships
			.memberships
			.iter()
			.find(|i| i.id.as_str() == "co")
			.unwrap()
			.state
			.iter(),
	)
	.then(|state| async { storage.get_value(&state.state).await })
	.map_ok(|state| state.into_value().1)
	.try_collect::<Vec<BTreeSet<Cid>>>()
	.await
	.unwrap();

	// check
	assert_eq!(heads.len(), 1);
	assert_eq!(heads.first().unwrap().len(), 1);
}
