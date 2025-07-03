use co_sdk::ApplicationBuilder;
use co_storage::TmpDir;

/// Create Local CO in tmpdir and exit.
#[tokio::test]
async fn test_local_multi_instance() {
	let tmp = TmpDir::new("co");

	// open first
	let application1 =
		ApplicationBuilder::new_with_path(format!("{}-test_local_multi_instance_1", tmp.uuid()), tmp.path().to_owned())
			.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			.with_optional_tracing()
			.without_keychain()
			.build()
			.await
			.expect("application");
	let local_co1 = application1.local_co_reducer().await.unwrap();
	let local_co1_state = local_co1.reducer_state().await;

	// open second
	let application2 =
		ApplicationBuilder::new_with_path(format!("{}-test_local_multi_instance_2", tmp.uuid()), tmp.path().to_owned())
			.without_keychain()
			.build()
			.await
			.expect("application");
	let local_co_2 = application2.local_co_reducer().await.unwrap();

	// the open of the second should not trigger any writes
	assert_eq!(local_co1_state, local_co1.reducer_state().await);
	assert_eq!(local_co1_state, local_co_2.reducer_state().await);
}
