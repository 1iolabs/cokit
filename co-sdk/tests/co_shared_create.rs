// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_core_co::CoAction;
use co_sdk::{tags, CreateCo, CO_CORE_NAME_CO};
use helper::instance::Instances;

pub mod helper;

#[tokio::test]
async fn test_co_shared_create() {
	let peer1 = Instances::new("test").create().await;

	// create identity
	let identity = peer1.create_identity().await;

	// create shared co
	let shared_co = peer1
		.application
		.create_co(identity.clone(), CreateCo::new("shared", None))
		.await
		.unwrap();

	// push
	shared_co
		.push(&identity, CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("hello": "world") })
		.await
		.unwrap();
}
