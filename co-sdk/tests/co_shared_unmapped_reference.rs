// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_core_co::CoAction;
use co_primitives::TagsAction;
use co_sdk::{tags, BlockStorageExt, CreateCo, CO_CORE_NAME_CO};
use helper::instance::Instances;

pub mod helper;

/// On a single application:
/// 1. create a public shared CO and store a block into it (using a tag)
/// 2. create an encrypted shared CO and store the same block into it
///
/// Step 2 currently fails with:
/// "Unmapped reference found _ while storing _. Are you sure you stored all children nodes?".
#[tokio::test]
async fn test_shared_co_store_block_via_tag_public_then_encrypted() {
	let mut instances = Instances::new("test_shared_co_store_block_via_tag_public_then_encrypted");
	let peer1 = instances.create().await;

	// create identity
	let identity = peer1.create_identity().await;

	// the same block is referenced from both COs
	let block_payload: Vec<u8> = b"some-block-payload".to_vec();

	// // store the block once at the application level
	// let block_cid = peer1.application.storage().set_serialized(&block_payload).await.unwrap();

	// 1. public shared co - reference the block via tag
	let public_co = peer1
		.application
		.create_co(identity.clone(), CreateCo::new("public", None).with_public(true))
		.await
		.unwrap();
	let block_cid = public_co.storage().set_serialized(&block_payload).await.unwrap();
	public_co
		.push(&identity, CO_CORE_NAME_CO, &CoAction::Tags { action: TagsAction::insert(tags!("block": block_cid)) })
		.await
		.unwrap();

	// 2. encrypted shared co - reference the same block via tag
	let encrypted_co = peer1
		.application
		.create_co(identity.clone(), CreateCo::new("encrypted", None).with_public(false))
		.await
		.unwrap();
	let block_cid = encrypted_co.storage().set_serialized(&block_payload).await.unwrap();
	let result = encrypted_co
		.push(&identity, CO_CORE_NAME_CO, &CoAction::Tags { action: TagsAction::insert(tags!("block": block_cid)) })
		.await;

	// check
	//  note: was error with "Unmapped reference found" before the fix for #125.
	assert!(result.is_ok());

	// let error = result.expect_err(
	// 	"expected encrypted shared CO to fail when referencing a block via tag that was first stored on a public CO",
	// );
	// let message = format!("{:#}", error);
	// assert!(
	// 	message.contains("Unmapped reference found") && message.contains("Are you sure you stored all children nodes?"),
	// 	"unexpected error message: {message}"
	// );
}
