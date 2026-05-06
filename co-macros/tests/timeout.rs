// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use std::time::Duration;

#[co_macros::timeout(1000)]
#[tokio::test]
async fn fast_async_body_passes() {
	tokio::time::sleep(Duration::from_millis(10)).await;
}

#[co_macros::timeout(50)]
#[tokio::test]
#[should_panic(expected = "test timed out after 50ms")]
async fn slow_async_body_times_out() {
	// Will be cancelled by timeout long before this completes,
	// so the test still finishes in ~50ms.
	tokio::time::sleep(Duration::from_secs(3600)).await;
}

#[co_macros::timeout(1000)]
#[tokio::test]
async fn result_returning_body_is_supported() -> Result<(), std::io::Error> {
	tokio::time::sleep(Duration::from_millis(10)).await;
	Ok(())
}
