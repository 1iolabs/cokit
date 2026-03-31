// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_log::{EntryBlock, Log};
use co_storage::BlockStorage;
use futures::Stream;
use std::collections::BTreeSet;

/// Read log entries from `newer` heads to  `older` heads.
/// The read starts with the latest entries in the log.
/// The `newer` heads are included.
/// The `older` heads are excluded.
/// Note: When `older` heads are never found the whole log will be retuned.
pub fn log_entries_until<S>(
	storage: S,
	newer: BTreeSet<Cid>,
	older: BTreeSet<Cid>,
) -> impl Stream<Item = Result<EntryBlock, anyhow::Error>>
where
	S: BlockStorage + Clone + 'static,
{
	async_stream::try_stream! {
		let mut stack_newer = newer.clone();
		let mut stack_older = older.clone();

		// walk entries from newest to oldest
		let log = Log::new_local(vec![], newer.clone());
		let entries = log.stream(&storage);
		for await entry in entries {
			let entry = entry?;

			// done when common ancestor found
			if stack_newer == stack_older {
				break;
			}

			// wals both stacks backward
			if stack_newer.remove(entry.cid()) {
				stack_newer.extend(entry.entry().next.iter().clone());
			}
			if stack_older.remove(entry.cid()) {
				stack_older.extend(entry.entry().next.iter().clone());
			}

			// if we have common heads we need to ignore it because all older heads should be excluded.
			if newer.contains(entry.cid()) && older.contains(entry.cid()) {
				continue;
			}

			yield entry;
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::library::log_entries_until::log_entries_until;
	use co_identity::LocalIdentity;
	use co_log::Log;
	use co_storage::{BlockStorageExt, MemoryBlockStorage};
	use futures::TryStreamExt;

	#[tokio::test]
	async fn test_log_entries_until() {
		// setup
		let storage = MemoryBlockStorage::default();
		let identity = LocalIdentity::device();
		let mut log = Log::new_local(Default::default(), Default::default());
		log.push_event(&storage, &identity, &String::from("a")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("b")).await.unwrap();
		let older = log.heads().clone();
		log.push_event(&storage, &identity, &String::from("c")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("d")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("e")).await.unwrap();
		let newer = log.heads().clone();
		log.push_event(&storage, &identity, &String::from("f")).await.unwrap();

		// call
		let entries = log_entries_until(storage.clone(), newer, older)
			.and_then(|entry| {
				let storage = storage.clone();
				async move { Ok(storage.get_deserialized::<String>(&entry.entry().payload).await.unwrap()) }
			})
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		assert_eq!(entries, ["e", "d", "c"].into_iter().map(String::from).collect::<Vec<String>>());
	}

	/// Test with conflicting (b1, b2).
	#[tokio::test]
	async fn test_overlap() {
		// setup
		let storage = MemoryBlockStorage::default();
		let identity = LocalIdentity::device();
		let mut log = Log::new_local(Default::default(), Default::default());
		log.push_event(&storage, &identity, &String::from("a")).await.unwrap();
		let mut log2 = Log::new_local(Default::default(), log.heads().clone());
		log2.push_event(&storage, &identity, &String::from("b2")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("b1")).await.unwrap();
		let older = log.heads().clone(); // b1
		log.join(&storage, &log2).await.unwrap(); // +b2
		let newer = log.heads().clone(); // b1, b2
		log.push_event(&storage, &identity, &String::from("c")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("d")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("e")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("f")).await.unwrap();

		// verify entry order
		let all_entries = log
			.stream(&storage)
			.and_then(|entry| {
				let storage = storage.clone();
				async move { Ok(storage.get_deserialized::<String>(&entry.entry().payload).await.unwrap()) }
			})
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		assert_eq!(
			all_entries,
			["f", "e", "d", "c", "b2", "b1", "a"]
				.into_iter()
				.map(String::from)
				.collect::<Vec<String>>()
		);

		// call
		let entries = log_entries_until(storage.clone(), newer, older)
			.and_then(|entry| {
				let storage = storage.clone();
				async move { Ok(storage.get_deserialized::<String>(&entry.entry().payload).await.unwrap()) }
			})
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		assert_eq!(entries, ["b2"].into_iter().map(String::from).collect::<Vec<String>>());
	}

	/// Test with conflicting but inversed merge (b2, b1).
	#[tokio::test]
	async fn test_overlap_inverse() {
		// setup
		let storage = MemoryBlockStorage::default();
		let identity = LocalIdentity::device();
		let mut log = Log::new_local(Default::default(), Default::default());
		log.push_event(&storage, &identity, &String::from("a")).await.unwrap();
		let mut log2 = Log::new_local(Default::default(), log.heads().clone());
		log2.push_event(&storage, &identity, &String::from("b1")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("b2")).await.unwrap();
		let older = log.heads().clone(); // b2
		log.join(&storage, &log2).await.unwrap(); // +b1
		let newer = log.heads().clone(); // b1, b2
		log.push_event(&storage, &identity, &String::from("c")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("d")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("e")).await.unwrap();
		log.push_event(&storage, &identity, &String::from("f")).await.unwrap();

		// verify entry order
		let all_entries = log
			.stream(&storage)
			.and_then(|entry| {
				let storage = storage.clone();
				async move { Ok(storage.get_deserialized::<String>(&entry.entry().payload).await.unwrap()) }
			})
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		assert_eq!(
			all_entries,
			["f", "e", "d", "c", "b2", "b1", "a"]
				.into_iter()
				.map(String::from)
				.collect::<Vec<String>>()
		);

		// call
		let entries = log_entries_until(storage.clone(), newer, older)
			.and_then(|entry| {
				let storage = storage.clone();
				async move { Ok(storage.get_deserialized::<String>(&entry.entry().payload).await.unwrap()) }
			})
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		assert_eq!(entries, ["b1"].into_iter().map(String::from).collect::<Vec<String>>());
	}

	#[tokio::test]
	async fn test_example() {
		// setup
		let storage = MemoryBlockStorage::default();
		let identity = LocalIdentity::device();
		let mut log1 = Log::new_local(Default::default(), Default::default());
		let mut log2 = Log::new_local(Default::default(), Default::default());
		let mut log3 = Log::new_local(Default::default(), Default::default());

		// create
		log2.push_event(&storage, &identity, &String::from("7")).await.unwrap();
		log1.join(&storage, &log2).await.unwrap();
		log3.join(&storage, &log2).await.unwrap();

		log3.push_event(&storage, &identity, &String::from("8")).await.unwrap();
		log1.join(&storage, &log3).await.unwrap();
		log2.join(&storage, &log3).await.unwrap();

		log2.push_event(&storage, &identity, &String::from("9")).await.unwrap();
		log1.join(&storage, &log2).await.unwrap();
		log3.join(&storage, &log2).await.unwrap();

		log1.push_event(&storage, &identity, &String::from("A")).await.unwrap();
		log2.join(&storage, &log1).await.unwrap();
		log3.join(&storage, &log1).await.unwrap();

		log1.push_event(&storage, &identity, &String::from("B")).await.unwrap();
		log2.join(&storage, &log1).await.unwrap();
		log3.join(&storage, &log1).await.unwrap();

		log3.push_event(&storage, &identity, &String::from("B*")).await.unwrap();

		log1.push_event(&storage, &identity, &String::from("C")).await.unwrap();
		log2.push_event(&storage, &identity, &String::from("C'")).await.unwrap();
		log2.join(&storage, &log1).await.unwrap();

		log1.push_event(&storage, &identity, &String::from("D")).await.unwrap();
		log2.join(&storage, &log1).await.unwrap();

		log2.push_event(&storage, &identity, &String::from("E")).await.unwrap();
		log1.join(&storage, &log2).await.unwrap();
		let older = log1.heads().clone(); // E

		log1.join(&storage, &log3).await.unwrap();
		log1.push_event(&storage, &identity, &String::from("F*")).await.unwrap();
		log2.join(&storage, &log1).await.unwrap();
		log3.join(&storage, &log1).await.unwrap();
		let newer = log1.heads().clone(); // F*

		// verify entry order
		async fn verify(storage: &MemoryBlockStorage, log: &Log) {
			let all_entries = log
				.stream(storage)
				.and_then(|entry| {
					let storage = storage.clone();
					async move { Ok(storage.get_deserialized::<String>(&entry.entry().payload).await.unwrap()) }
				})
				.try_collect::<Vec<_>>()
				.await
				.unwrap();
			assert_eq!(
				all_entries,
				["F*", "E", "D", "C", "C'", "B*", "B", "A", "9", "8", "7"]
					.into_iter()
					.map(String::from)
					.collect::<Vec<String>>()
			);
		}
		verify(&storage, &log1).await;
		verify(&storage, &log2).await;
		verify(&storage, &log3).await;

		// call
		let entries = log_entries_until(storage.clone(), newer, older)
			.and_then(|entry| {
				let storage = storage.clone();
				async move { Ok(storage.get_deserialized::<String>(&entry.entry().payload).await.unwrap()) }
			})
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		assert_eq!(
			entries,
			["F*", "E", "D", "C", "C'", "B*"]
				.into_iter()
				.map(String::from)
				.collect::<Vec<String>>()
		);
	}
}
