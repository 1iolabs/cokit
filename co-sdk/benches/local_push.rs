use co_core_co::CoAction;
use co_sdk::{build_core, crate_repository_path, Application, ApplicationBuilder, CO_CORE_NAME_CO};
use criterion::{criterion_group, criterion_main, Criterion};
use example_counter::CounterAction;
use tokio::runtime::Builder;

async fn setup_local_memory() -> Application {
	let application = ApplicationBuilder::new_memory("test".to_owned())
		// .with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
		.without_keychain()
		.build()
		.await
		.expect("application");
	let local_co = application.local_co_reducer().await.unwrap();
	let counter = build_core(crate_repository_path(true).unwrap(), "examples/counter")
		.unwrap()
		.store_artifact(&local_co.storage())
		.await
		.unwrap();
	local_co
		.push(
			&application.local_identity(),
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
		)
		.await
		.unwrap();
	application
}

fn local_push_benchmark(c: &mut Criterion) {
	let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
	let application = runtime.block_on(setup_local_memory());
	c.bench_function("local_push", move |b| b.to_async(&runtime).iter(|| local_push(application.clone())));
}

async fn local_push(application: Application) {
	let local_co = application.local_co_reducer().await.unwrap();
	local_co
		.push(&application.local_identity(), "counter", &CounterAction::Increment(1))
		.await
		.unwrap();
}

criterion_group!(benches, local_push_benchmark);
criterion_main!(benches);
