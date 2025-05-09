use co_sdk::{
	state::{self, query_core, QueryExt},
	ApplicationBuilder, DidKeyIdentity, Identity, MonotonicCoDate, MonotonicCoUuid, CO_CORE_NAME_KEYSTORE,
};
use co_storage::TmpDir;
use std::collections::BTreeMap;

pub mod helper;

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
	let (storage, key_store) = query_core::<co_core_keystore::KeyStore>(CO_CORE_NAME_KEYSTORE)
		.execute_reducer(&local_co)
		.await
		.unwrap();
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
	let (storage, key_store) = query_core::<co_core_keystore::KeyStore>(CO_CORE_NAME_KEYSTORE)
		.execute_reducer(&local_co)
		.await
		.unwrap();
	let keys: BTreeMap<String, co_core_keystore::Key> =
		state::into_collection(&storage, &key_store.keys).await.unwrap();
	let key = keys.get(identity.identity()).expect("identity");
	assert_eq!(key, &identity.export().unwrap());
}
