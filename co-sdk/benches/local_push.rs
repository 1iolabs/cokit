// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_core_co::CoAction;
use co_runtime::Core;
use co_sdk::{build_core, crate_repository_path, Application, ApplicationBuilder, BuildCoreArtifact, CO_CORE_NAME_CO};
use co_storage::MemoryBlockStorage;
use co_test::test_log_path;
use criterion::{criterion_group, criterion_main, Criterion};
use example_counter::{Counter, CounterAction};
use tokio::runtime::Builder;

async fn build_counter(native: bool) -> (Cid, Core, BuildCoreArtifact) {
	let core_storage = MemoryBlockStorage::default();
	let repository_path = crate_repository_path(true).unwrap();
	let core_path = repository_path.join("examples/counter");
	let counter_artifact = build_core(repository_path, core_path).unwrap();
	let counter = counter_artifact.store_artifact(&core_storage).await.unwrap();
	let native_counter = Core::native::<Counter>();
	(counter, if native { native_counter } else { Core::Wasm(counter) }, counter_artifact)
}

async fn setup_local_memory() -> Application {
	// core
	let (counter, counter_core, counter_artifact) = build_counter(false).await;

	// application
	let application = ApplicationBuilder::new_memory("test".to_owned())
		.with_bunyan_logging(Some(test_log_path()))
		.with_optional_tracing()
		.without_keychain()
		.with_core(counter, counter_core)
		.build()
		.await
		.expect("application");

	// co
	let local_co = application.local_co_reducer().await.unwrap();
	counter_artifact.store_artifact(&local_co.storage()).await.unwrap();
	local_co
		.push(
			&application.local_identity(),
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
		)
		.await
		.unwrap();

	// result
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
