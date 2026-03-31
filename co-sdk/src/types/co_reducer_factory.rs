// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::CoReducer;
use async_trait::async_trait;
use co_actor::ActorError;
use co_primitives::CoId;
use std::time::Duration;

#[async_trait]
pub trait CoReducerFactory {
	/// Get instance of CoReducer.
	/// Returns None if `co` membership could not be found.
	/// TODO: Refactor to own error type and remove option.
	async fn co_reducer(&self, co: &CoId) -> Result<Option<CoReducer>, anyhow::Error>;

	async fn try_co_reducer(&self, co: &CoId) -> Result<CoReducer, CoReducerFactoryError>;

	async fn try_co_reducer_with_options(
		&self,
		co: &CoId,
		options: CoOptions,
	) -> Result<CoReducer, CoReducerFactoryError>;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CoOptions {
	/// Wait for CO to become active.
	///
	/// See:
	/// - [`co_core_membership::MembershipState::Pending`]
	/// - [`co_core_membership::MembershipState::Join`]
	pub wait: bool,

	/// Optional wait timeout before to fail.
	pub wait_timeout: Option<Duration>,
}
impl CoOptions {
	pub fn with_wait(mut self, timeout: Option<Duration>) -> Self {
		self.wait = true;
		self.wait_timeout = timeout;
		self
	}
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CoReducerFactoryError {
	#[error("CO not found: {0:?}")]
	CoNotFound(CoId, #[source] anyhow::Error),

	#[error("Create CO failed: {0:?}")]
	Create(CoId, #[source] anyhow::Error),

	#[error("CO failed")]
	Other(#[from] anyhow::Error),

	#[error("CO actor error")]
	Actor(#[from] ActorError),

	#[error("CO not initialized yet")]
	WouldCreate,
}

pub trait CoReducerFactoryResultExt<T> {
	/// Return None for pending/uninitialized COs.
	fn opt(self) -> Result<Option<T>, CoReducerFactoryError>;
}
impl<T> CoReducerFactoryResultExt<T> for Result<T, CoReducerFactoryError> {
	fn opt(self) -> Result<Option<T>, CoReducerFactoryError> {
		match self {
			Err(CoReducerFactoryError::WouldCreate) => Ok(None),
			Ok(value) => Ok(Some(value)),
			Err(err) => Err(err),
		}
	}
}
