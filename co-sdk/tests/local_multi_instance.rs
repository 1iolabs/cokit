use co_core_co::CoAction;
use co_primitives::CoJoin;
use co_sdk::{tags, ApplicationBuilder, KnownTag, Tags, CO_CORE_NAME_CO};
use co_storage::TmpDir;
use futures::{pin_mut, StreamExt};

/// Create Local CO in tmpdir open a second instance and exit.
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
	let local_co2 = application2.local_co_reducer().await.unwrap();

	// the open of the second should not trigger any writes
	assert_eq!(local_co1_state, local_co1.reducer_state().await);
	assert_eq!(local_co1_state, local_co2.reducer_state().await);
}

/// Create Local CO in tmpdir open a second instance, push someting and exit.
#[tokio::test]
async fn test_local_multi_instance_push() {
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
	let identity = application1.local_identity();
	let local_co1 = application1.local_co_reducer().await.unwrap();
	let local_co1_state = local_co1.reducer_state().await;
	tracing::info!(?local_co1_state, "test-open");

	// open second
	let application2 =
		ApplicationBuilder::new_with_path(format!("{}-test_local_multi_instance_2", tmp.uuid()), tmp.path().to_owned())
			.without_keychain()
			.build()
			.await
			.expect("application");
	let local_co2 = application2.local_co_reducer().await.unwrap();
	let local_co2_state = local_co2.reducer_state().await;
	tracing::info!(?local_co2_state, "test-open");

	// setup wait
	let local_co2_next_state = tokio::spawn({
		let stream = local_co2.reducer_state_stream().skip(1).take(1).inspect(|state| {
			tracing::info!(?state, "test-push-change");
		});
		async move {
			pin_mut!(stream);
			let result = stream.next().await.expect("state");
			tracing::info!(?result, "test-push-done");
			result
		}
	});

	// push
	let push_state = local_co1
		.push(&identity, CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("hello": "world") })
		.await
		.unwrap();
	let local_co1_state = local_co1.reducer_state().await;
	tracing::info!(?push_state, ?local_co1_state, "test-push");
	assert_eq!(local_co1_state, local_co2_next_state.await.unwrap());
}

/// Create Local CO in tmpdir open a second instance, push someting and exit.
#[tokio::test]
async fn test_local_co_tags() {
	let tmp = TmpDir::new("co");

	// open first
	let application =
		ApplicationBuilder::new_with_path(format!("{}-test_local_co_tags", tmp.uuid()), tmp.path().to_owned())
			.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			.with_optional_tracing()
			.without_keychain()
			.build()
			.await
			.expect("application");

	let local_co = application.local_co_reducer().await.expect("local co");

	let mut tags = Tags::new();
	tags.insert(CoJoin::Accept.tag());
	local_co
		.push(&application.local_identity(), CO_CORE_NAME_CO, &CoAction::TagsInsert { tags })
		.await
		.unwrap();
}
