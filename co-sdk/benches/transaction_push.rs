// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_core_co::CoAction;
use co_identity::LocalIdentity;
use co_runtime::Core;
use co_sdk::{
	build_core, crate_repository_path, Application, ApplicationBuilder, BuildCoreArtifact, CoReducer,
	CO_CORE_NAME_CO,
};
use co_storage::MemoryBlockStorage;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use example_counter::{Counter, CounterAction};
use tokio::runtime::Builder;

async fn build_counter() -> (Cid, Core, BuildCoreArtifact) {
	let core_storage = MemoryBlockStorage::default();
	let repository_path = crate_repository_path(true).unwrap();
	let core_path = repository_path.join("examples/counter");
	let counter_artifact = build_core(repository_path, core_path).unwrap();
	let counter = counter_artifact.store_artifact(&core_storage).await.unwrap();
	let native_counter = Core::native::<Counter, CounterAction>();
	(counter, native_counter, counter_artifact)
}

async fn setup() -> (Application, CoReducer) {
	let (counter, counter_core, counter_artifact) = build_counter().await;
	let application = ApplicationBuilder::new_memory("bench_transaction".to_owned())
		.without_keychain()
		.with_disabled_feature("co-local-encryption")
		.with_core(counter, counter_core)
		.build()
		.await
		.expect("application");
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
	(application, local_co)
}

async fn sequential_push(local_co: &CoReducer, count: usize) {
	let identity = LocalIdentity::new("bench");
	for index in 0..count {
		local_co.push(&identity, "counter", &CounterAction::Increment(index as i64)).await.unwrap();
	}
}

async fn transaction_push(local_co: &CoReducer, count: usize) {
	let identity = LocalIdentity::new("bench");
	let mut tx = local_co.transaction(identity).unwrap();
	for index in 0..count {
		tx.push("counter", &CounterAction::Increment(index as i64)).await.unwrap();
	}
	tx.commit().await.unwrap();
}

fn benchmark(c: &mut Criterion) {
	let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

	let mut group = c.benchmark_group("push_batch");
	for count in [10, 100, 1000] {
		let (_app_seq, co_seq) = runtime.block_on(setup());
		group.bench_with_input(BenchmarkId::new("sequential", count), &count, |b, &count| {
			b.to_async(&runtime).iter(|| sequential_push(&co_seq, count));
		});

		let (_app_tx, co_tx) = runtime.block_on(setup());
		group.bench_with_input(BenchmarkId::new("transaction", count), &count, |b, &count| {
			b.to_async(&runtime).iter(|| transaction_push(&co_tx, count));
		});
	}
	group.finish();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
