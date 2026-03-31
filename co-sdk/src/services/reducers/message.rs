// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::ReducerStorage;
use crate::{types::co_reducer_factory::CoReducerFactoryError, CoOptions, CoReducer};
use co_actor::{ActorHandle, Response};
use co_primitives::CoId;

#[derive(Debug)]
pub enum ReducerRequest {
	/// Request CO storage instance (without networking).
	Storage(CoId, ReducerOptions, Response<Result<ReducerStorage, CoReducerFactoryError>>),
	/// Request CO reducer instance by creating it if not created yet.
	Request(CoId, ReducerOptions, Response<Result<CoReducer, CoReducerFactoryError>>),
	/// Create reducer instance.
	Create(CoId, Result<CoReducer, CoReducerFactoryError>),
	/// Create shared storage instance.
	CreateStorage(CoId, Result<ReducerStorage, CoReducerFactoryError>),
	/// Clear all reducer instances.
	Clear(Response<Result<(), CoReducerFactoryError>>),
	/// Clear a specific reducer instance.
	ClearOne(CoId, Response<Result<(), CoReducerFactoryError>>),
	/// Test if a CO instance is running already.
	IsRunning(CoId, Response<bool>),
}

#[derive(Clone)]
pub struct ReducersControl {
	pub(crate) handle: ActorHandle<ReducerRequest>,
}
impl ReducersControl {
	pub async fn is_running(&self, co: CoId) -> bool {
		self.handle
			.request(|response| ReducerRequest::IsRunning(co, response))
			.await
			.unwrap_or_default()
	}

	pub async fn storage(&self, co: CoId, options: ReducerOptions) -> Result<ReducerStorage, CoReducerFactoryError> {
		// tracing::trace!(?co, err = ?anyhow::anyhow!("test"), "co-reducer-request");
		Ok(self
			.handle
			.try_request(|response| ReducerRequest::Storage(co, options, response))
			.await?)
	}

	pub async fn reducer(&self, co: CoId, options: ReducerOptions) -> Result<CoReducer, CoReducerFactoryError> {
		// tracing::trace!(?co, err = ?anyhow::anyhow!("test"), "co-reducer-request");
		Ok(self
			.handle
			.try_request(|response| ReducerRequest::Request(co, options, response))
			.await?)
	}

	pub async fn create(&self, co: CoId, reducer: Result<CoReducer, CoReducerFactoryError>) {
		self.handle.dispatch(ReducerRequest::Create(co, reducer)).ok();
	}

	pub async fn create_storage(&self, co: CoId, storage: Result<ReducerStorage, CoReducerFactoryError>) {
		self.handle.dispatch(ReducerRequest::CreateStorage(co, storage)).ok();
	}

	pub async fn clear(&self) -> Result<(), CoReducerFactoryError> {
		Ok(self.handle.try_request(ReducerRequest::Clear).await?)
	}

	pub async fn clear_one(&self, co: CoId) -> Result<(), CoReducerFactoryError> {
		Ok(self
			.handle
			.try_request(|response| ReducerRequest::ClearOne(co, response))
			.await?)
	}
}
impl From<ActorHandle<ReducerRequest>> for ReducersControl {
	fn from(value: ActorHandle<ReducerRequest>) -> Self {
		Self { handle: value }
	}
}

#[derive(Debug, Clone, Default)]
pub struct ReducerOptions {
	/// When set to [`true`] do not attempt to create and return [`None`]/[`CoReducerFactoryError::WouldCreate`].
	pub no_create: bool,

	/// Co Options
	pub co: CoOptions,
}
impl ReducerOptions {
	pub fn with_no_create(mut self) -> Self {
		self.no_create = true;
		self
	}

	pub fn with_co_options(mut self, co: CoOptions) -> Self {
		self.co = co;
		self
	}
}
