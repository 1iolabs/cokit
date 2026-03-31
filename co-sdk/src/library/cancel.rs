// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use futures::Future;
use tokio_util::sync::CancellationToken;

#[allow(dead_code)]
pub async fn cancel<F, R>(cancel: CancellationToken, fut: F) -> Option<R>
where
	F: Future<Output = Option<R>>,
{
	tokio::select! {
		_ = cancel.cancelled() => None,
		v = fut => v,
	}
}

#[allow(dead_code)]
pub async fn try_cancel<F, R, E>(cancel: CancellationToken, fut: F) -> Result<Option<R>, E>
where
	F: Future<Output = Result<Option<R>, E>>,
{
	tokio::select! {
		_ = cancel.cancelled() => Ok(None),
		v = fut => v,
	}
}
