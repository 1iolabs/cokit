use cid::Cid;
use co_core_co::CoAction;
use co_sdk::{
	build_core, crate_repository_path,
	state::{query_core, QueryExt},
	ApplicationBuilder, BlockStorage, BlockStorageExt, MonotonicCoDate, MonotonicCoUuid, CO_CORE_NAME_CO,
};

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

async fn counter_upgraded_core<S>(storage: &S) -> Cid
where
	S: BlockStorage + 'static,
{
	let counter = build_core(crate_repository_path(true).unwrap(), "examples/counter-upgraded")
		.unwrap()
		.store_artifact(storage)
		.await
		.unwrap();
	counter
}

/// Upgrades from `example_counter` to `example_counter_upgraded` and verifes the migration take place.
#[tokio::test]
async fn test_core_upgrade() {
	// app
	let application_identifier = format!("test_core_upgrade-{}", uuid::Uuid::new_v4().to_string());
	let application = ApplicationBuilder::new_memory(application_identifier)
		.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
		.with_optional_tracing()
		.without_keychain()
		.with_disabled_feature("co-local-encryption")
		.with_co_date(MonotonicCoDate::default())
		.with_co_uuid(MonotonicCoUuid::default())
		.build()
		.await
		.expect("application");
	let counter = counter_core(&application.storage()).await;
	println!("counter {:?}", counter);
	let counter_upgraded = counter_upgraded_core(&application.storage()).await;
	println!("counter_upgraded {:?}", counter_upgraded);
	let local_identity = application.local_identity();
	let local_co = application.local_co_reducer().await.unwrap();

	// create
	local_co
		.push(
			&local_identity,
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
		)
		.await
		.unwrap();

	// write
	local_co
		.push(&local_identity, "counter", &example_counter::CounterAction::Increment(100))
		.await
		.unwrap();

	// upgrade
	local_co
		.push(
			&local_identity,
			CO_CORE_NAME_CO,
			&CoAction::CoreUpgrade {
				core: "counter".to_owned(),
				binary: counter_upgraded,
				migrate: Some(
					local_co
						.storage()
						.set_serialized(&example_counter_upgraded::CounterAction::MigrateFromV1)
						.await
						.unwrap(),
				),
			},
		)
		.await
		.unwrap();

	// check
	let counter: example_counter_upgraded::Counter = query_core("counter").execute_reducer(&local_co).await.unwrap().1;
	assert_eq!(counter, example_counter_upgraded::Counter { count: 100 });
}
