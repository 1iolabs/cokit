use cid::Cid;
use co_core_co::CoAction;
use co_sdk::{
	build_core, crate_repository_path,
	state::{query_core, QueryExt},
	Action, AnyBlockStorage, ApplicationBuilder, CoReducer, CoreName, MonotonicCoDate, MonotonicCoUuid,
	CO_CORE_NAME_CO, CO_ID_LOCAL,
};
use co_test::{test_log_path, test_tmp_dir};
use example_counter::{Counter, CounterAction};
use futures::StreamExt;
use ipld_core::serde::from_ipld;
use std::future::ready;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

async fn counter_core(storage: &impl AnyBlockStorage) -> Cid {
	let repository_path = crate_repository_path(true).unwrap();
	let core_path = repository_path.join("examples/counter");
	let counter = build_core(repository_path, core_path)
		.unwrap()
		.store_artifact(storage)
		.await
		.unwrap();
	counter
}

async fn counter_count(co: &CoReducer) -> i64 {
	let (_storage, counter) = query_core(CoreName::<Counter>::new("counter"))
		.execute_reducer(co)
		.await
		.unwrap();
	counter.0
}

#[tokio::test]
async fn test_local_join() {
	// app
	let application_identifier = format!("test_local_join-{}", uuid::Uuid::new_v4());
	let tmp = test_tmp_dir().without_clear();
	let application1 = ApplicationBuilder::new_with_path(application_identifier.clone(), tmp.path().to_owned())
		.with_bunyan_logging(Some(test_log_path()))
		.with_optional_tracing()
		.without_keychain()
		.with_disabled_feature("co-local-watch")
		.with_disabled_feature("co-local-encryption")
		.with_co_date(MonotonicCoDate::default())
		.with_co_uuid(MonotonicCoUuid::default())
		.build()
		.await
		.expect("application");
	let counter = counter_core(&application1.storage()).await;
	let local_identity = application1.local_identity();
	let local_co1 = application1.local_co_reducer().await.unwrap();
	local_co1
		.push(
			&local_identity,
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
		)
		.await
		.unwrap();

	// push
	for i in 0..3 {
		local_co1
			.push(&local_identity, "counter", &CounterAction::Increment(i + 1))
			.await
			.unwrap();
	}
	assert_eq!(counter_count(&local_co1).await, 6);

	// open second instance
	let application2 = ApplicationBuilder::new_with_path(format!("{application_identifier}_2"), tmp.path().to_owned())
		.without_keychain()
		.with_disabled_feature("co-local-watch")
		.with_disabled_feature("co-local-encryption")
		.with_co_date(MonotonicCoDate::default())
		.with_co_uuid(MonotonicCoUuid::default())
		.build()
		.await
		.expect("application");
	let local_co2 = application2.local_co_reducer().await.unwrap();
	assert_eq!(local_co1.reducer_state().await, local_co2.reducer_state().await);

	// push
	local_co1
		.push(&local_identity, "counter", &CounterAction::Increment(10))
		.await
		.unwrap();
	assert_eq!(counter_count(&local_co1).await, 16);

	// open third instance
	let application3 = ApplicationBuilder::new_with_path(format!("{application_identifier}_3"), tmp.path().to_owned())
		.without_keychain()
		.with_disabled_feature("co-local-watch")
		.with_disabled_feature("co-local-encryption")
		.with_co_date(MonotonicCoDate::default())
		.with_co_uuid(MonotonicCoUuid::default())
		.build()
		.await
		.expect("application");
	let local_co3 = application3.local_co_reducer().await.unwrap();
	assert_eq!(local_co1.reducer_state().await, local_co3.reducer_state().await);

	// push conflict
	local_co2
		.push(&local_identity, "counter", &CounterAction::Increment(1))
		.await
		.unwrap();
	assert_eq!(counter_count(&local_co2).await, 7);

	// listen for actions applied
	let init = CancellationToken::new();
	let done = CancellationToken::new();
	let (tx, rx) = oneshot::channel();
	tokio::spawn({
		let application3 = application3.clone();
		let done = done.child_token();
		let init = init.clone();
		async move {
			// started
			init.cancel();

			// collect
			let actions = application3
				.actions()
				.filter_map(|action| {
					ready(match action {
						Action::CoreAction { co, action, .. }
							if co.as_str() == CO_ID_LOCAL && &action.core == "counter" =>
						{
							println!("action: {:?}", action);
							Some(from_ipld::<CounterAction>(action.payload).unwrap())
						},
						Action::Error { err } => {
							panic!("action error: {:?}", err);
						},
						_ => None,
					})
				})
				.take_until(done.cancelled_owned())
				.collect::<Vec<_>>()
				.await;

			// result
			tx.send(actions).ok();
		}
	});
	init.cancelled().await;

	// update thrid with conflicting application
	local_co3.join_state(local_co2.reducer_state().await).await.unwrap();
	done.cancel();
	let actions = rx.await.unwrap();
	assert_eq!(actions.len(), 2);
	assert_eq!(actions, vec![CounterAction::Increment(10), CounterAction::Increment(1)]);
	assert_eq!(counter_count(&local_co3).await, 17);
}
