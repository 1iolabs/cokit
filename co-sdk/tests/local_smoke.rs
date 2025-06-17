use cid::Cid;
use co_core_co::CoAction;
use co_sdk::{
	build_core, crate_repository_path,
	state::{self, query_core, QueryExt},
	ApplicationBuilder, BlockStorage, DidKeyIdentity, Identity, MonotonicCoDate, MonotonicCoUuid, CO_CORE_NAME_CO,
	CO_CORE_NAME_KEYSTORE,
};
use co_storage::TmpDir;
use example_counter::CounterAction;
use std::collections::BTreeMap;

pub mod helper;

async fn counter_core<S>(storage: &S) -> Cid
where
	S: BlockStorage + 'static,
{
	let counter = build_core(crate_repository_path(true).unwrap(), "examples/counter")
		.unwrap()
		.store_artifact(storage)
		.await
		.unwrap();
	counter
}

/// Create Local CO in tmpdir and exit.
/// This test is designed to not have random values and should therefore always use the same Cids.
#[tokio::test]
async fn test_local_smoke() {
	let tmp = TmpDir::new("co");

	// create
	let identity = DidKeyIdentity::generate(Some(&vec![1; 32]));
	{
		let application = ApplicationBuilder::new_with_path("test".to_owned(), tmp.path().to_owned())
			.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			.with_optional_tracing()
			.without_keychain()
			.with_disabled_feature("co-local-encryption")
			.with_co_date(MonotonicCoDate::default())
			.with_co_uuid(MonotonicCoUuid::default())
			.build()
			.await
			.expect("application");
		let local_identity = application.local_identity();
		let local_co = application.local_co_reducer().await.unwrap();
		local_co
			.push(
				&local_identity,
				CO_CORE_NAME_KEYSTORE,
				&co_core_keystore::KeyStoreAction::Set(identity.export().unwrap()),
			)
			.await
			.unwrap();
	}

	// reopen
	let application = ApplicationBuilder::new_with_path("test".to_owned(), tmp.path().to_owned())
		.without_keychain()
		.with_disabled_feature("co-local-encryption")
		.with_co_date(MonotonicCoDate::default())
		.with_co_uuid(MonotonicCoUuid::default())
		.build()
		.await
		.expect("application");
	let local_co = application.local_co_reducer().await.unwrap();
	let (storage, key_store) = query_core(CO_CORE_NAME_KEYSTORE).execute_reducer(&local_co).await.unwrap();
	let keys: BTreeMap<String, co_core_keystore::Key> =
		state::into_collection(&storage, &key_store.keys).await.unwrap();
	let key = keys.get(identity.identity()).expect("identity");
	assert_eq!(key, &identity.export().unwrap());
}

/// Create Local CO in tmpdir and exit.
#[tokio::test]
async fn test_local_smoke_encrypted() {
	let tmp = TmpDir::new("co");

	// create
	let identity = DidKeyIdentity::generate(None);
	{
		let application = ApplicationBuilder::new_with_path("test".to_owned(), tmp.path().to_owned())
			.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			.with_optional_tracing()
			.without_keychain()
			.build()
			.await
			.expect("application");
		let local_identity = application.local_identity();
		let local_co = application.local_co_reducer().await.unwrap();
		local_co
			.push(
				&local_identity,
				CO_CORE_NAME_KEYSTORE,
				&co_core_keystore::KeyStoreAction::Set(identity.export().unwrap()),
			)
			.await
			.unwrap();
	}

	// reopen
	let application = ApplicationBuilder::new_with_path("test".to_owned(), tmp.path().to_owned())
		.without_keychain()
		.build()
		.await
		.expect("application");
	let local_co = application.local_co_reducer().await.unwrap();
	let (storage, key_store) = query_core(CO_CORE_NAME_KEYSTORE).execute_reducer(&local_co).await.unwrap();
	let keys: BTreeMap<String, co_core_keystore::Key> =
		state::into_collection(&storage, &key_store.keys).await.unwrap();
	let key = keys.get(identity.identity()).expect("identity");
	assert_eq!(key, &identity.export().unwrap());
}

/// Create Local CO in tmpdir and exit.
/// This test is designed to not have random values and should therefore always use the same Cids.
#[tokio::test]
async fn test_local_push() {
	// app
	let application_identifier = format!("test_local_push-{}", uuid::Uuid::new_v4().to_string());
	let tmp = TmpDir::new("co");
	let application = ApplicationBuilder::new_with_path(application_identifier, tmp.path().to_owned())
		.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
		.with_optional_tracing()
		.without_keychain()
		.with_disabled_feature("co-local-encryption")
		// .with_setting("feature", "co-storage-free")
		.with_co_date(MonotonicCoDate::default())
		.with_co_uuid(MonotonicCoUuid::default())
		.build()
		.await
		.expect("application");
	let counter = counter_core(&application.storage()).await;
	println!("counter {:?}", counter);
	let local_identity = application.local_identity();
	let local_co = application.local_co_reducer().await.unwrap();
	local_co
		.push(
			&local_identity,
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
		)
		.await
		.unwrap();

	// push
	for i in 0..4 {
		local_co
			.push(&application.local_identity(), "counter", &CounterAction::Increment(i))
			.await
			.unwrap();
	}
}

/// Create Local CO in tmpdir and exit.
#[tokio::test]
async fn test_local_push_encrypted() {
	// app
	let application_identifier = format!("test_local_push_encrypted-{}", uuid::Uuid::new_v4().to_string());
	let tmp = TmpDir::new("co");
	let application = ApplicationBuilder::new_with_path(application_identifier, tmp.path().to_owned())
		.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
		.with_optional_tracing()
		.with_bunyan_logging(None)
		.without_keychain()
		.build()
		.await
		.expect("application");
	let local_co = application.local_co_reducer().await.unwrap();
	let counter = counter_core(&local_co.storage()).await;
	println!("counter {:?}", counter);
	let local_identity = application.local_identity();
	local_co
		.push(
			&local_identity,
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
		)
		.await
		.unwrap();

	// push
	for i in 0..4 {
		local_co
			.push(&application.local_identity(), "counter", &CounterAction::Increment(i))
			.await
			.unwrap();
	}
}
