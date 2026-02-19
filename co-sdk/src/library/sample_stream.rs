// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use futures::{pin_mut, Stream};
use std::{
	cmp::Ordering,
	collections::{hash_map::DefaultHasher, BinaryHeap},
	hash::{Hash, Hasher},
};
use tokio_stream::StreamExt;

/// Deterministic streaming sampler.
/// That preserves order, guarantees first + last to be included and returns exactly k items if input length >= k.
pub async fn sample_stream_ordered_first_last<S, T>(stream: S, k: usize) -> Result<Vec<T>, anyhow::Error>
where
	S: Stream<Item = Result<T, anyhow::Error>> + 'static,
{
	pin_mut!(stream);

	// validate
	if k == 0 {
		return Ok(Vec::new());
	}

	// first
	let first = match stream.try_next().await? {
		Some(x) if k == 1 => return Ok(vec![x]),
		Some(x) => x,
		None => return Ok(Vec::new()),
	};

	// second element (for 1-item lag)
	let mut previous = match stream.try_next().await? {
		Some(x) => x,
		None => return Ok(vec![first]),
	};

	let mid_cap = k.saturating_sub(2);
	let mut heap: BinaryHeap<Entry<T>> = BinaryHeap::with_capacity(mid_cap);

	let mut index: u64 = 1;

	while let Some(current) = stream.try_next().await? {
		if mid_cap > 0 {
			let score = deterministic_score(index);

			if heap.len() < mid_cap {
				heap.push(Entry { score, index, item: previous });
			} else if let Some(worst) = heap.peek() {
				if score < worst.score || (score == worst.score && index < worst.index) {
					heap.pop();
					heap.push(Entry { score, index, item: previous });
				}
			}
		}

		index += 1;
		previous = current;
	}

	// last element
	let last = previous;
	let last_idx = index;

	// collect and restore order
	let mut picked: Vec<(u64, T)> = Vec::with_capacity(2 + heap.len());
	picked.push((0, first));
	picked.extend(heap.into_iter().map(|entry| (entry.index, entry.item)));
	picked.push((last_idx, last));

	picked.sort_by_key(|(i, _)| *i);
	Ok(picked.into_iter().map(|(_, x)| x).collect())
}

#[derive(Debug)]
struct Entry<T> {
	score: u64, // lower is better
	index: u64,
	item: T,
}
impl<T> Ord for Entry<T> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.score.cmp(&other.score).then_with(|| self.index.cmp(&other.index))
	}
}
impl<T> PartialOrd for Entry<T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
impl<T> PartialEq for Entry<T> {
	fn eq(&self, other: &Self) -> bool {
		self.score == other.score && self.index == other.index
	}
}
impl<T> Eq for Entry<T> {}

fn deterministic_score(idx: u64) -> u64 {
	let mut hasher = DefaultHasher::new();
	idx.hash(&mut hasher);
	hasher.finish()
}

#[cfg(test)]
mod tests {
	use super::sample_stream_ordered_first_last;
	use futures::stream;

	#[tokio::test]
	async fn test_sample_stream_ordered_first_last() {
		let values = (0..10).map(|i| format!("value_{}", i));

		// test with k=0
		let samples = sample_stream_ordered_first_last(stream::iter(values.clone().map(Ok)), 0)
			.await
			.unwrap();
		assert!(samples.is_empty());

		// test with k=3
		let samples = sample_stream_ordered_first_last(stream::iter(values.clone().map(Ok)), 3)
			.await
			.unwrap();
		assert_eq!(samples.len(), 3);
		assert_eq!(samples[0], "value_0");
		assert_eq!(samples[1], "value_1");
		assert_eq!(samples[2], "value_9");

		// test with k=5
		let samples = sample_stream_ordered_first_last(stream::iter(values.clone().map(Ok)), 5)
			.await
			.unwrap();
		assert_eq!(samples.len(), 5);
		assert_eq!(samples[0], "value_0");
		assert_eq!(samples[1], "value_1");
		assert_eq!(samples[2], "value_4");
		assert_eq!(samples[3], "value_8");
		assert_eq!(samples[4], "value_9");

		// test with k=10 (all)
		let samples = sample_stream_ordered_first_last(stream::iter(values.clone().map(Ok)), 10)
			.await
			.unwrap();
		assert_eq!(samples.len(), 10);
		assert_eq!(samples[0], "value_0");
		assert_eq!(samples[1], "value_1");
		assert_eq!(samples[2], "value_2");
		assert_eq!(samples[3], "value_3");
		assert_eq!(samples[4], "value_4");
		assert_eq!(samples[5], "value_5");
		assert_eq!(samples[6], "value_6");
		assert_eq!(samples[7], "value_7");
		assert_eq!(samples[8], "value_8");
		assert_eq!(samples[9], "value_9");

		// test with single element stream and k=1
		let samples = sample_stream_ordered_first_last(stream::iter(["single_value"].into_iter().map(Result::Ok)), 1)
			.await
			.unwrap();
		assert_eq!(samples.len(), 1);
		assert_eq!(samples[0], "single_value");
	}
}
