use cid::Cid;
use co_core_co::CoAction;
use co_identity::LocalIdentity;
use co_primitives::MonotonicCoDate;
use co_sdk::{
	build_core, crate_repository_path,
	state::{query_core, QueryExt},
	Action, AnyBlockStorage, ApplicationBuilder, BlockStorageExt, CoReducer, CoreName, MonotonicCoUuid, ReducerAction,
	CO_CORE_NAME_CO, CO_ID_LOCAL,
};
use co_test::{test_log_path, test_tmp_dir};
use example_counter::{Counter, CounterAction};
use futures::{StreamExt, TryStreamExt};
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
	let local_identity1 = LocalIdentity::new("app1");
	let local_identity2 = LocalIdentity::new("app2");
	let local_co1 = application1.local_co_reducer().await.unwrap();
	local_co1
		.push(
			&local_identity1,
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
		)
		.await
		.unwrap();

	// push
	for i in 0..3 {
		local_co1
			.push(&local_identity1, "counter", &CounterAction::Increment(i + 1))
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
		.push(&local_identity1, "counter", &CounterAction::Increment(10))
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
		.push(&local_identity2, "counter", &CounterAction::Increment(1))
		.await
		.unwrap();
	assert_eq!(counter_count(&local_co2).await, 7);

	// listen for actions applied
	let init = CancellationToken::new();
	let done = CancellationToken::new();
	let (tx, rx) = oneshot::channel();
	application3.context().tasks().spawn({
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
	assert_eq!(counter_count(&local_co3).await, 17);
	done.cancel();
	let actions = rx.await.unwrap();
	assert_eq!(actions.len(), 2);
	assert!(actions.contains(&CounterAction::Increment(10)));
	assert!(actions.contains(&CounterAction::Increment(1)));

	// check actual order
	// note:
	// 	checking order is actually tricky because every change in history (like the Cid of the core)
	//  influences the ordereing when using the same identity
	//  as this cahnges the Cid any therefore the deterministic sorting
	//  normally that is not a problem because we use the same core but here we compare different
	//  cores (build on different machines) for the same order
	let (storage, entries) = application3.context().entries(local_co3.id()).await.unwrap();
	let mut counter_actions = entries
		.try_filter_map(|entry_block| {
			let storage = storage.clone();
			async move {
				Ok(storage
					.get_deserialized::<ReducerAction<CounterAction>>(&entry_block.entry().payload)
					.await
					.ok()
					.filter(|reducer_action| reducer_action.core == "counter")
					.map(|reducer_action| reducer_action.payload))
			}
		})
		.try_collect::<Vec<_>>()
		.await
		.unwrap();
	counter_actions.reverse();
	assert_eq!(
		counter_actions,
		vec![
			CounterAction::Increment(1),
			CounterAction::Increment(2),
			CounterAction::Increment(3),
			CounterAction::Increment(10),
			CounterAction::Increment(1),
		]
	)
}
