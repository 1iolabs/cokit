// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use tokio::sync::mpsc::error::SendError;
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
	#[error("Shutdown in progress. Operation canceled.")]
	Shutdown,
}
impl<T> From<SendError<T>> for NetworkError {
	fn from(_: SendError<T>) -> Self {
		Self::Shutdown
	}
}
