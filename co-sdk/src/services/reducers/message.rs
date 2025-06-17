use super::ReducerStorage;
use crate::{types::co_reducer_factory::CoReducerFactoryError, CoReducer};
use co_actor::{ActorHandle, Response};
use co_primitives::CoId;

#[derive(Debug)]
pub enum ReducerRequest {
	/// Request CO storage instance (without networking).
	Storage(CoId, Response<Result<ReducerStorage, CoReducerFactoryError>>),
	/// Request CO reducer instance by creating it if not created yet.
	Request(CoId, Response<Result<CoReducer, CoReducerFactoryError>>),
	/// Request CO reducer instance if it already has been created yet.
	RequestOpt(CoId, Response<Option<CoReducer>>),
	/// Create reducer instance.
	Create(CoId, Result<CoReducer, CoReducerFactoryError>),
	/// Create shared storage instance.
	CreateStorage(CoId, Result<ReducerStorage, CoReducerFactoryError>),
	/// Clear all reducer instances.
	Clear(Response<Result<(), CoReducerFactoryError>>),
	/// Clear a specific reducer instance.
	ClearOne(CoId, Response<Result<(), CoReducerFactoryError>>),
}

#[derive(Clone)]
pub struct ReducersControl {
	pub(crate) handle: ActorHandle<ReducerRequest>,
}
impl ReducersControl {
	pub async fn storage(&self, co: CoId) -> Result<ReducerStorage, CoReducerFactoryError> {
		// tracing::trace!(?co, err = ?anyhow::anyhow!("test"), "co-reducer-request");
		Ok(self
			.handle
			.try_request(|response| ReducerRequest::Storage(co, response))
			.await?)
	}

	pub async fn reducer(&self, co: CoId) -> Result<CoReducer, CoReducerFactoryError> {
		// tracing::trace!(?co, err = ?anyhow::anyhow!("test"), "co-reducer-request");
		Ok(self
			.handle
			.try_request(|response| ReducerRequest::Request(co, response))
			.await?)
	}

	pub async fn reducer_opt(&self, co: CoId) -> Option<CoReducer> {
		self.handle
			.request(|response| ReducerRequest::RequestOpt(co, response))
			.await
			.ok()?
	}

	pub async fn create(&self, co: CoId, reducer: Result<CoReducer, CoReducerFactoryError>) {
		self.handle.dispatch(ReducerRequest::Create(co, reducer)).ok();
	}

	pub async fn create_storage(&self, co: CoId, storage: Result<ReducerStorage, CoReducerFactoryError>) {
		self.handle.dispatch(ReducerRequest::CreateStorage(co, storage)).ok();
	}

	pub async fn clear(&self) -> Result<(), CoReducerFactoryError> {
		Ok(self.handle.try_request(|response| ReducerRequest::Clear(response)).await?)
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
