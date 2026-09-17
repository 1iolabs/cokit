// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#![cfg(not(feature = "js"))]

use cid::Cid;
use co_core_rich_text::{
	Attributes, AttributesOperation, DeleteAction, FormatAction, InsertAction, InsertionPoint, Position, RichText,
	RichTextAction,
};
use co_identity::{Identity, IdentityResolver, LocalIdentity, LocalIdentityResolver};
use co_log::{IdentityEntryVerifier, Log};
use co_primitives::ReducerInput;
use co_runtime::RuntimeActor;
use co_sdk::{
	build_core, crate_repository_path, BlockStorageExt, Core, Date, Link, MemoryBlockStorage, ReducerAction,
	RuntimeContext, RuntimeHandle,
};
use futures::TryStreamExt;
use std::collections::BTreeSet;

/// The freshly built rich-text WASM core next to the native reducer.
struct Cores {
	storage: MemoryBlockStorage,
	runtime: RuntimeHandle,
	binary: Cid,
	wasm: Core,
	native: Core,
}
impl Cores {
	async fn build() -> Self {
		let storage = MemoryBlockStorage::default();
		let repository_path = crate_repository_path(true).unwrap();
		let binary = build_core(&repository_path, repository_path.join("cores/rich-text"))
			.unwrap()
			.store_artifact(&storage)
			.await
			.unwrap();
		println!("rich-text core {binary}");
		let runtime = RuntimeActor::spawn("rich_text_wasm_parity", Default::default()).unwrap();
		Self { storage, runtime, binary, wasm: binary.into(), native: Core::native::<RichText, RichTextAction>() }
	}

	// execute one action; a rejected action keeps the state and returns the reducer error
	async fn reduce(&self, core: &Core, state: Option<Cid>, action: Cid) -> Result<Option<Cid>, String> {
		let context = RuntimeContext::new(&ReducerInput { state, action }).unwrap();
		let context = self
			.runtime
			.execute_state(&self.storage, &self.binary, core, context)
			.await
			.unwrap();
		match context.ok() {
			Ok(()) => Ok(context.state),
			Err(error) => {
				assert_eq!(context.state, state, "{action}");
				Err(error.to_string())
			},
		}
	}

	// replay through both cores, requiring identical acceptance and state after every action
	async fn replay(&self, actions: &[Cid]) -> (Option<Cid>, Vec<bool>) {
		let mut state = None;
		let mut accepted = Vec::new();
		for action in actions {
			let wasm = self.reduce(&self.wasm, state, *action).await;
			let native = self.reduce(&self.native, state, *action).await;
			match (&wasm, &native) {
				(Ok(wasm), Ok(native)) => {
					assert_eq!(wasm, native, "{action}");
					state = *wasm;
					accepted.push(true);
				},
				(Err(wasm), Err(native)) => {
					assert_eq!(wasm, native, "{action}");
					accepted.push(false);
				},
				_ => panic!("{action}: wasm {wasm:?}, native {native:?}"),
			}
		}
		(state, accepted)
	}

	async fn state(&self, state: Option<Cid>) -> RichText {
		match state {
			Some(cid) => self.storage.get_value(&Link::<RichText>::new(cid)).await.unwrap(),
			None => RichText::default(),
		}
	}

	async fn plain_text(&self, state: Option<Cid>) -> String {
		self.state(state).await.plain_text(&self.storage).await.unwrap()
	}

	async fn positions(&self, state: Option<Cid>) -> Vec<Position> {
		self.state(state)
			.await
			.chars(self.storage.clone())
			.map_ok(|(_char, position, _attributes)| position)
			.try_collect::<Vec<_>>()
			.await
			.unwrap()
	}

	async fn action(&self, from: &str, time: &mut Date, payload: impl Into<RichTextAction>) -> Cid {
		let action = ReducerAction { core: "".to_owned(), from: from.to_owned(), payload: payload.into(), time: *time };
		*time += 1;
		*self.storage.set_value(&action).await.unwrap().cid()
	}

	// apply one more action to `actions`, replaying everything through both cores
	async fn step(&self, actions: &mut Vec<Cid>, time: &mut Date, payload: impl Into<RichTextAction>) -> Option<Cid> {
		actions.push(self.action("", time, payload).await);
		let (state, accepted) = self.replay(actions).await;
		assert!(accepted.iter().all(|accepted| *accepted));
		state
	}
}

fn insert(at: InsertionPoint, text: &str) -> InsertAction {
	InsertAction { at, text: text.to_owned(), attributes: Default::default() }
}

/// A mixed action history replays to the same states in WASM and native execution,
/// and both cores reject empty insertion text at every insertion point.
#[tokio::test]
async fn test_mixed_history_replays_identically() {
	let cores = Cores::build().await;
	let mut time = 1;
	let mut actions = Vec::new();
	let bold = AttributesOperation::Merge(Attributes::default().with_attribute("bold", true));
	let italic = AttributesOperation::Replace(Attributes::default().with_attribute("italic", true));

	let _ = cores
		.step(&mut actions, &mut time, insert(InsertionPoint::Start, "hello"))
		.await;
	let state = cores
		.step(
			&mut actions,
			&mut time,
			InsertAction { at: InsertionPoint::End, text: " world".to_owned(), attributes: bold.clone() },
		)
		.await;
	let p = cores.positions(state).await;
	let state = cores
		.step(&mut actions, &mut time, insert(InsertionPoint::Before(p[6]), "big "))
		.await;
	let p = cores.positions(state).await;
	let state = cores
		.step(&mut actions, &mut time, insert(InsertionPoint::Before(p[2]), "|"))
		.await;
	let p = cores.positions(state).await;
	let state = cores
		.step(&mut actions, &mut time, DeleteAction { at: p[3], last: Some(p[6]) })
		.await;
	let p = cores.positions(state).await;
	let _ = cores
		.step(&mut actions, &mut time, FormatAction { at: p[3], last: Some(p[5]), attributes: italic })
		.await;
	let _ = cores
		.step(&mut actions, &mut time, InsertAction { at: InsertionPoint::End, text: "!".to_owned(), attributes: bold })
		.await;
	let state = cores.step(&mut actions, &mut time, insert(InsertionPoint::Start, "é中")).await;

	assert_eq!(cores.plain_text(state).await, "é中he|big world!");

	let p = cores.positions(state).await;
	for at in [InsertionPoint::Start, InsertionPoint::End, InsertionPoint::Before(p[0]), InsertionPoint::After(p[0])] {
		let mut rejected = actions.clone();
		rejected.push(cores.action("", &mut time, insert(at.clone(), "")).await);
		let (rejected_state, accepted) = cores.replay(&rejected).await;
		assert_eq!(accepted.last(), Some(&false), "{at:?}");
		assert_eq!(rejected_state, state, "{at:?}");
	}
}

/// Valid and invalid `After` anchors are accepted and rejected alike by WASM and native replay.
#[tokio::test]
async fn test_after_parity() {
	let cores = Cores::build().await;
	let mut time = 1;
	let text = "Aé中😀B";
	let base = vec![cores.action("", &mut time, insert(InsertionPoint::Start, text)).await];
	let (base_state, _accepted) = cores.replay(&base).await;
	let positions = cores.positions(base_state).await;
	let id = positions[0];

	// visible anchors, including run splits and the end
	for (anchor, expected) in [(0, "A|é中😀B"), (1, "Aé|中😀B"), (2, "Aé中|😀B"), (3, "Aé中😀|B"), (4, "Aé中😀B|")]
	{
		let mut actions = base.clone();
		actions.push(
			cores
				.action("", &mut time, insert(InsertionPoint::After(positions[anchor]), "|"))
				.await,
		);
		let (state, accepted) = cores.replay(&actions).await;
		assert_eq!(accepted, [true, true]);
		assert_eq!(cores.plain_text(state).await, expected);
	}

	// deleted anchor
	let mut actions = base.clone();
	actions.push(cores.action("", &mut time, DeleteAction { at: positions[2], last: None }).await);
	actions.push(
		cores
			.action("", &mut time, insert(InsertionPoint::After(positions[2]), "|"))
			.await,
	);
	let (state, accepted) = cores.replay(&actions).await;
	assert_eq!(accepted, [true, true, true]);
	assert_eq!(cores.plain_text(state).await, "Aé|😀B");

	// invalid anchors and empty text are rejected without changing the state
	let mut invalid: Vec<InsertAction> = [2, 4, 5, 7, 8, 9, 11]
		.map(|offset| insert(InsertionPoint::After(id.right_by(offset)), "|"))
		.into();
	invalid.push(insert(InsertionPoint::After(id), ""));
	for action in invalid {
		let mut actions = base.clone();
		actions.push(cores.action("", &mut time, action.clone()).await);
		let (state, accepted) = cores.replay(&actions).await;
		assert_eq!(accepted, [true, false], "{action:?}");
		assert_eq!(state, base_state);
	}
}

fn signed_log(heads: BTreeSet<Cid>) -> Log {
	Log::new(b"rich-text".to_vec(), IdentityEntryVerifier::new(LocalIdentityResolver::new().boxed()), heads)
}

// type `text` behind the visible text, one scalar per signed entry
async fn type_after(cores: &Cores, log: &mut Log, identity: &LocalIdentity, time: &mut Date, text: &str) {
	for char in text.chars() {
		let (state, _accepted) = cores.replay(&log_actions(cores, log).await).await;
		let at = cores
			.positions(state)
			.await
			.last()
			.map_or(InsertionPoint::Start, |last| InsertionPoint::After(*last));
		let action = ReducerAction {
			core: "".to_owned(),
			from: identity.identity().to_owned(),
			payload: RichTextAction::from(insert(at, &char.to_string())),
			time: *time,
		};
		*time += 1;
		log.push_event(&cores.storage, identity, &action).await.unwrap();
	}
}

// the log's actions, oldest first
async fn log_actions(cores: &Cores, log: &Log) -> Vec<Cid> {
	let mut entries = log.stream(&cores.storage).try_collect::<Vec<_>>().await.unwrap();
	entries.reverse();
	entries.into_iter().map(|entry| entry.entry().payload).collect()
}

/// Two peers typing behind shared text while disconnected merge to contiguous sequences with one state CID,
/// in WASM and native replay, in both join directions, and when reopened from the merged heads.
#[tokio::test]
async fn test_merged_replay_parity() {
	let cores = Cores::build().await;
	let mut time = 1;
	let a = LocalIdentityResolver::new().private_identity("did:local:a").unwrap();
	let b = LocalIdentityResolver::new().private_identity("did:local:b").unwrap();

	let mut log_a = signed_log(Default::default());
	type_after(&cores, &mut log_a, &a, &mut time, "ABC").await;
	let mut log_b = signed_log(log_a.heads().clone());
	type_after(&cores, &mut log_a, &a, &mut time, "xy").await;
	type_after(&cores, &mut log_b, &b, &mut time, "uv").await;

	let mut merged_a = log_a.clone();
	assert!(merged_a.join(&cores.storage, &log_b).await.unwrap());
	let mut merged_b = log_b.clone();
	assert!(merged_b.join(&cores.storage, &log_a).await.unwrap());
	let (state, accepted) = cores.replay(&log_actions(&cores, &merged_a).await).await;
	assert_eq!(accepted.len(), 7);
	assert!(accepted.iter().all(|accepted| *accepted));
	let text = cores.plain_text(state).await;
	assert!(["ABCxyuv", "ABCuvxy"].contains(&text.as_str()), "{text}");
	assert_eq!(cores.replay(&log_actions(&cores, &merged_b).await).await.0, state);
	let reopened = signed_log(merged_a.heads().clone());
	assert_eq!(cores.replay(&log_actions(&cores, &reopened).await).await.0, state);
}
