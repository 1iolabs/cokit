// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_core_co::CoAction;
use co_runtime::Core;
use co_sdk::{
	build_core, crate_repository_path, Application, ApplicationBuilder, BuildCoreArtifact, CoId, CoReducerFactory,
	CreateCo, DidKeyIdentity, DidKeyProvider, PrivateIdentity, PrivateIdentityBox, CO_CORE_NAME_CO,
	CO_CORE_NAME_KEYSTORE,
};
use co_storage::MemoryBlockStorage;
use co_test::TmpDir;
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

async fn setup_memory(public: bool) -> (Application, PrivateIdentityBox) {
	// core
	let (counter, counter_core, counter_artifact) = build_counter(true).await;

	let application = ApplicationBuilder::new_memory("test".to_owned())
		.without_keychain()
		.with_core(counter, counter_core)
		.build()
		.await
		.expect("application");
	let local_co = application.local_co_reducer().await.unwrap();

	// identity
	let identity = DidKeyIdentity::generate(None);
	let provider = DidKeyProvider::new(local_co, CO_CORE_NAME_KEYSTORE);
	provider.store(&identity, None).await.unwrap();

	// create co
	let co = application
		.create_co(identity.clone(), CreateCo::new("shared", None).with_public(public))
		.await
		.unwrap();
	counter_artifact.store_artifact(&co.storage()).await.unwrap();
	co.push(
		&identity,
		CO_CORE_NAME_CO,
		&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
	)
	.await
	.unwrap();

	// result
	(application, identity.boxed())
}

async fn setup_file(public: bool) -> (Application, PrivateIdentityBox, TmpDir) {
	let tmp = TmpDir::new("co");
	let application = ApplicationBuilder::new_with_path("test", tmp.path().to_owned())
		.without_keychain()
		.build()
		.await
		.expect("application");
	let local_co = application.local_co_reducer().await.unwrap();

	// identity
	let identity = DidKeyIdentity::generate(None);
	let provider = DidKeyProvider::new(local_co, CO_CORE_NAME_KEYSTORE);
	provider.store(&identity, None).await.unwrap();

	// create co
	let co = application
		.create_co(identity.clone(), CreateCo::new("shared", None).with_public(public))
		.await
		.unwrap();
	let repository_path = crate_repository_path(true).unwrap();
	let core_path = repository_path.join("examples/counter");
	let counter = build_core(repository_path, core_path)
		.unwrap()
		.store_artifact(&co.storage())
		.await
		.unwrap();
	co.push(
		&identity,
		CO_CORE_NAME_CO,
		&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
	)
	.await
	.unwrap();

	// result
	(application, identity.boxed(), tmp)
}

async fn shared_push(co_id: CoId, application: Application, identity: PrivateIdentityBox) {
	let co = application.context().try_co_reducer(&co_id).await.unwrap();
	co.push(&identity, "counter", &CounterAction::Increment(1)).await.unwrap();
}

fn benchmark(c: &mut Criterion) {
	let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
	let (application, identity) = runtime.block_on(setup_memory(false));
	let co_id = CoId::from("shared");
	c.bench_function("shared_push", move |b| {
		b.to_async(&runtime)
			.iter(|| shared_push(co_id.clone(), application.clone(), identity.clone()))
	});
}

fn public_benchmark(c: &mut Criterion) {
	let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
	let (application, identity) = runtime.block_on(setup_memory(true));
	let co_id = CoId::from("shared");
	c.bench_function("shared_push (public)", move |b| {
		b.to_async(&runtime)
			.iter(|| shared_push(co_id.clone(), application.clone(), identity.clone()))
	});
}

fn file_benchmark(c: &mut Criterion) {
	let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
	let (application, identity, _tmp) = runtime.block_on(setup_file(false));
	let co_id = CoId::from("shared");
	c.bench_function("shared_push (file)", move |b| {
		b.to_async(&runtime)
			.iter(|| shared_push(co_id.clone(), application.clone(), identity.clone()))
	});
}

fn file_public_benchmark(c: &mut Criterion) {
	let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
	let (application, identity, _tmp) = runtime.block_on(setup_file(true));
	let co_id = CoId::from("shared");
	c.bench_function("shared_push (file; public)", move |b| {
		b.to_async(&runtime)
			.iter(|| shared_push(co_id.clone(), application.clone(), identity.clone()))
	});
}

criterion_group!(benches, benchmark, public_benchmark, file_benchmark, file_public_benchmark);
criterion_main!(benches);
