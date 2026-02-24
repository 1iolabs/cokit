// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_core_co::CoAction;
use co_sdk::{tags, ApplicationBuilder, CO_CORE_NAME_CO};
use co_test::{test_log_path, test_tmp_dir, TmpDir};
use futures::{pin_mut, StreamExt};

/// Create Local CO in tmpdir open a second instance and exit.
#[tokio::test]
async fn test_local_multi_instance() {
	let tmp = test_tmp_dir();

	// open first
	let application1 =
		ApplicationBuilder::new_with_path(format!("{}-test_local_multi_instance_1", tmp.uuid()), tmp.path().to_owned())
			.with_bunyan_logging(Some(test_log_path()))
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
			.with_bunyan_logging(Some(test_log_path()))
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
