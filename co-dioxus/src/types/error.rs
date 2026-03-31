// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_actor::ActorError;
use std::{fmt::Debug, sync::Arc};

#[derive(Clone, thiserror::Error)]
#[error(transparent)]
pub struct CoError(Arc<anyhow::Error>);
impl CoError {
	pub fn new<E: Into<anyhow::Error>>(error: E) -> Self {
		Self(Arc::new(error.into()))
	}
}
impl From<anyhow::Error> for CoError {
	fn from(err: anyhow::Error) -> Self {
		Self::new(err)
	}
}
impl From<ActorError> for CoError {
	fn from(err: ActorError) -> Self {
		Self::new(err)
	}
}
impl Debug for CoError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}
