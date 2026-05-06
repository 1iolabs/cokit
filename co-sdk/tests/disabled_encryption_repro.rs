// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH
//
// Repro for: with_disabled_feature("co-local-encryption") + runtime
// CoAction::CoreCreate on a freshly-created CO deadlocks.
//
// Compare with core_upgrade.rs which exercises the same flag with CoreCreate
// on the LOCAL CO and passes — the bug is specific to pushing CoreCreate on
// a CO created via `application.create_co(...)`.

use co_core_co::CoAction;
use co_primitives::MonotonicCoDate;
use co_sdk::{ApplicationBuilder, BlockStorageExt, CreateCo, MonotonicCoUuid, CO_CORE_NAME_CO};
use co_storage::MemoryBlockStorage;
use co_test::test_log_path;

const MARKER: &str = "repro-binary-marker";

#[co_test::timeout(10000)]
#[tokio::test]
async fn core_create_on_fresh_co_with_disabled_encryption_hangs() {
	// Compute the CID of the marker bytes. Storing the same bytes in the CO's
	// own storage later produces the same CID (deterministic), so the binary
	// reference in CoreCreate resolves locally without needing a real wasm.
	let temp = MemoryBlockStorage::default();
	let binary = temp.set_serialized(&MARKER).await.unwrap();

	let id = format!("repro-{}", uuid::Uuid::new_v4());
	let application = ApplicationBuilder::new_memory(id)
		.with_bunyan_logging(Some(test_log_path()))
		.with_optional_tracing()
		.without_keychain()
		.with_disabled_feature("co-local-encryption")
		.with_co_date(MonotonicCoDate::default())
		.with_co_uuid(MonotonicCoUuid::default())
		.build()
		.await
		.unwrap();
	let identity = application.local_identity();

	// Create a fresh CO with no cores attached. This is the path co-profile uses.
	let create = CreateCo::generate("test-fresh-co".to_owned());
	let reducer = application.create_co(identity.clone(), create).await.unwrap();

	// Stash the binary marker in the new CO's storage so the binary CID resolves
	// locally, then push CoreCreate.
	reducer.storage().set_serialized(&MARKER).await.unwrap();

	// THIS is the call that hangs forever in the broken configuration.
	reducer
		.push(
			&identity,
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate { core: "test".to_owned(), binary, tags: Default::default() },
		)
		.await
		.expect("CoreCreate on a fresh CO must not deadlock");
}
