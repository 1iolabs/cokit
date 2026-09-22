// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_core_co::CoAction;
use co_primitives::TagsAction;
use co_sdk::{tags, ApplicationBuilder, CO_CORE_NAME_CO};
use co_test::{test_tmp_dir, TmpDir};
use futures::{pin_mut, StreamExt};

/// Create Local CO in tmpdir open a second instance and exit.
#[tokio::test]
async fn test_local_multi_instance() {
	co_test::init_test_log();
	let tmp = test_tmp_dir();

	// open first
	let application1 =
		ApplicationBuilder::new_with_path(format!("{}-test_local_multi_instance_1", tmp.uuid()), tmp.path().to_owned())
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
#[co_test::timeout(10000)]
#[tokio::test]
async fn test_local_multi_instance_push() {
	co_test::init_test_log();
	let tmp = TmpDir::new("co");

	// open first
	let application1 =
		ApplicationBuilder::new_with_path(format!("{}-test_local_multi_instance_1", tmp.uuid()), tmp.path().to_owned())
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
	let local_co2_next_state = application1.context().tasks().spawn({
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
		.push(&identity, CO_CORE_NAME_CO, &CoAction::Tags { action: TagsAction::insert(tags!("hello": "world")) })
		.await
		.unwrap();
	let local_co1_state = local_co1.reducer_state().await;
	tracing::info!(?push_state, ?local_co1_state, "test-push");
	assert_eq!(local_co1_state, local_co2_next_state.await.unwrap());
}

/// Pushes between two instances must not grow the process descriptor table while both watchers stay
/// alive. The parent re-executes this test binary with a private marker so the child alone lowers
/// the process-wide soft descriptor limit.
#[cfg(target_os = "macos")]
#[co_test::timeout(60000)]
#[tokio::test]
async fn test_local_multi_instance_descriptors_bounded() {
	const CHILD_MARKER: &str = "CO_SDK_TEST_LOCAL_DESCRIPTORS_CHILD";
	if std::env::var_os(CHILD_MARKER).is_some() {
		return tokio::time::timeout(std::time::Duration::from_secs(30), descriptors_bounded_child())
			.await
			.expect("descriptor child timed out");
	}

	let output = tokio::process::Command::new(std::env::current_exe().expect("test binary"))
		.args(["--exact", "test_local_multi_instance_descriptors_bounded", "--nocapture"])
		.env(CHILD_MARKER, "1")
		.output()
		.await
		.expect("spawn test binary");
	assert!(
		output.status.success(),
		"descriptor child failed ({}):\n{}\n{}",
		output.status,
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);
}

/// Child of [`test_local_multi_instance_descriptors_bounded`]: a configured `/var` root, two
/// instances, both watch directions warmed, then synchronized pushes between two descriptor counts.
#[cfg(target_os = "macos")]
async fn descriptors_bounded_child() {
	const WARM_PUSHES: usize = 4;
	const MEASURED_PUSHES: usize = 16;

	// no test log: its file writes pace the watcher and hide descriptor growth

	// lower only the soft limit; the hard limit stays untouched
	let mut limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
	assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) }, 0, "getrlimit");
	limit.rlim_cur = 256;
	assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) }, 0, "setrlimit");

	// the configured root is spelled `/var`; macOS reports it as `/private/var`
	let tmp = TmpDir::new("co");
	let configured = tmp.path().to_owned();
	assert!(configured.starts_with("/var"), "temp dir must be under /var: {configured:?}");
	let canonical = tokio::fs::canonicalize(&configured).await.expect("canonicalize");
	assert!(canonical.starts_with("/private/var"), "canonical root must be under /private/var: {canonical:?}");
	assert_ne!(configured, canonical);

	// both slot directories exist before either watcher starts
	let identifier1 = format!("{}-descriptors-1", tmp.uuid());
	let identifier2 = format!("{}-descriptors-2", tmp.uuid());
	for identifier in [&identifier1, &identifier2] {
		tokio::fs::create_dir_all(configured.join("etc").join(identifier))
			.await
			.expect("slot directory");
	}

	// open both
	let application1 = ApplicationBuilder::new_with_path(identifier1, configured.clone())
		.without_keychain()
		.build()
		.await
		.expect("application 1");
	let application2 = ApplicationBuilder::new_with_path(identifier2, configured)
		.without_keychain()
		.build()
		.await
		.expect("application 2");
	let identity1 = application1.local_identity();
	let identity2 = application2.local_identity();
	let local_co1 = application1.local_co_reducer().await.unwrap();
	let local_co2 = application2.local_co_reducer().await.unwrap();
	let changes1 = local_co1.reducer_state_stream();
	let changes2 = local_co2.reducer_state_stream();
	pin_mut!(changes1, changes2);

	// warm both directions so lazily opened descriptors belong to the baseline count
	for i in 0..WARM_PUSHES {
		push_and_observe(&local_co2, &identity2, &mut changes1, format!("warm-2-{i}")).await;
		push_and_observe(&local_co1, &identity1, &mut changes2, format!("warm-1-{i}")).await;
	}

	// measure while both applications and watchers stay alive
	let before = open_descriptors();
	for i in 0..MEASURED_PUSHES {
		push_and_observe(&local_co2, &identity2, &mut changes1, format!("push-2-{i}")).await;
		push_and_observe(&local_co1, &identity1, &mut changes2, format!("push-1-{i}")).await;
	}
	let after = open_descriptors();
	println!("descriptors before: {before}, after: {after}");
	assert!(after <= before, "descriptor table grew from {before} to {after} while both watchers were alive");
}

/// Push one tag into `pusher` and wait until `observer` reports the pusher's resulting state.
#[cfg(target_os = "macos")]
async fn push_and_observe<S>(
	pusher: &co_sdk::CoReducer,
	identity: &co_identity::LocalIdentity,
	observer: &mut S,
	value: String,
) where
	S: futures::Stream + Unpin,
	S::Item: PartialEq<co_sdk::CoReducerState> + std::fmt::Debug,
{
	pusher
		.push(identity, CO_CORE_NAME_CO, &CoAction::Tags { action: TagsAction::insert(tags!("count": value)) })
		.await
		.unwrap();
	let expected = pusher.reducer_state().await;
	while let Some(state) = observer.next().await {
		if state == expected {
			return;
		}
	}
	panic!("observer stream ended before delivering {expected:?}");
}

/// Number of open descriptors of this process.
#[cfg(target_os = "macos")]
fn open_descriptors() -> usize {
	std::fs::read_dir("/dev/fd").expect("/dev/fd").count()
}
