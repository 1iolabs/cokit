// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_primitives::{LsmTreeMap, LsmTreeMapSettings, TestStorage};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::cell::RefCell;
use tokio::runtime::Builder;

#[allow(clippy::await_holding_refcell_ref)]
fn lsm_tree_map_benchmark(c: &mut Criterion) {
	c.bench_function("insert_and_get", |b| {
		let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
		let tree = RefCell::new(runtime.block_on(async {
			let storage = TestStorage::default();
			let settings = LsmTreeMapSettings { max_node_entries: 32, max_active_entries: 2, max_run_count: 2 };
			LsmTreeMap::new(storage.clone(), settings)
		}));
		let mut i = 1;
		b.to_async(&runtime).iter_batched(
			|| {
				i += 1;
				i
			},
			|i| {
				let i = black_box(i);
				let tree = &tree;
				async move {
					tree.borrow_mut().insert(i, 0).await.unwrap();
					tree.borrow_mut().get(&i).await.unwrap();
				}
			},
			criterion::BatchSize::SmallInput,
		)
	});
}

criterion_group!(benches, lsm_tree_map_benchmark);
criterion_main!(benches);
