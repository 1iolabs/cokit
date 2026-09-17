// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_api::{BlockStorageExt, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction};
use co_core_rich_text::{InsertAction, InsertionPoint, RichText, RichTextAction, TextModel, TextModelChange};
use co_storage::MemoryBlockStorage;
use futures::{FutureExt, TryStreamExt};

// apply one action to `state` through the public reducer, as an application outside the crate would
async fn dispatch(
	storage: &MemoryBlockStorage,
	state: OptionLink<RichText>,
	action: RichTextAction,
) -> anyhow::Result<Link<RichText>> {
	let action = ReducerAction { core: String::new(), from: String::new(), payload: action, time: 1 };
	let action_link = storage.set_value(&action).await?;
	RichText::reduce(state, action_link, &CoreBlockStorage::new(storage.clone(), true))
		.boxed()
		.await
}

#[tokio::test]
async fn test_text_model_from_absent_state() {
	let storage = MemoryBlockStorage::default();
	let model = TextModel::new(storage.clone(), OptionLink::none());

	assert_eq!(model.plain_text().await.unwrap(), "");
	assert!(model.insert(0, String::new(), Default::default()).await.is_err());

	let action = model.insert(0, "hi".to_owned(), Default::default()).await.unwrap();
	match &action {
		RichTextAction::Insert(insert) => assert_eq!(insert.at, InsertionPoint::Start),
		_ => panic!("expected insert action"),
	}

	match model.text_change(std::slice::from_ref(&action)).await.unwrap().as_slice() {
		[TextModelChange::Insert { index, text, .. }] => {
			assert_eq!(*index, 0);
			assert_eq!(text.as_str(), "hi");
		},
		other => panic!("expected one insert change: {other:?}"),
	}

	let state_link = dispatch(&storage, OptionLink::none(), action).await.unwrap();
	let model = TextModel::new(storage.clone(), state_link.into());
	assert_eq!(model.plain_text().await.unwrap(), "hi");
}

#[tokio::test]
async fn test_text_model_from_stored_state() {
	let storage = MemoryBlockStorage::default();
	let source_text = "Aé中😀B";
	let state_link = dispatch(
		&storage,
		OptionLink::none(),
		InsertAction { at: InsertionPoint::Start, text: source_text.to_owned(), attributes: Default::default() }.into(),
	)
	.await
	.unwrap();
	let state = storage.get_value(&state_link).await.unwrap();
	let positions = state
		.chars(storage.clone())
		.map_ok(|(_char, position, _attributes)| position)
		.try_collect::<Vec<_>>()
		.await
		.unwrap();

	let model = TextModel::new(storage.clone(), state_link.into());
	assert_eq!(model.plain_text().await.unwrap(), source_text);

	// byte index 3 sits between the two-byte 'é' and the three-byte '中'; the insert anchors after 'é'
	let insert_after = model.insert(3, "|".to_owned(), Default::default()).await.unwrap();
	match &insert_after {
		RichTextAction::Insert(insert) => assert_eq!(insert.at, InsertionPoint::After(positions[1])),
		_ => panic!("expected insert action"),
	}
	match model.text_change(std::slice::from_ref(&insert_after)).await.unwrap().as_slice() {
		[TextModelChange::Insert { index, text, .. }] => {
			assert_eq!(*index, 3);
			assert_eq!(text.as_str(), "|");
		},
		other => panic!("expected one insert change: {other:?}"),
	}

	// byte index 2 is the second byte of 'é', not a scalar boundary
	assert!(model.insert(2, "|".to_owned(), Default::default()).await.is_err());
	assert!(model.insert(3, String::new(), Default::default()).await.is_err());
}
