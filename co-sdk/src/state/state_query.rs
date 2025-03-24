use crate::{state, CoReducer, CoStorage};
use co_core_co::Co;
use co_primitives::OptionLink;
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use serde::de::DeserializeOwned;
use std::future::Future;

/// Fluent query interface for cores.
pub struct StateQuery<S, T> {
	storage: S,
	state_reference: OptionLink<T>,
	state: Option<T>,
}
impl<S, T> StateQuery<S, T>
where
	S: BlockStorage + Clone + 'static,
	T: Send + Sync + Clone + DeserializeOwned + 'static,
{
	pub fn storage(&self) -> &S {
		&self.storage
	}

	pub fn state_reference(&self) -> OptionLink<T> {
		self.state_reference
	}

	pub fn is_none(&self) -> bool {
		self.state_reference.is_none()
	}

	pub async fn state(&mut self) -> Result<Option<&T>, StateQueryError> {
		// fetch
		if self.state.is_none() {
			if let Some(link) = self.state_reference.link() {
				self.state = Some(self.storage.get_value(&link).await?);
			}
		}

		// result
		Ok(self.state.as_ref())
	}

	pub async fn state_or_default(&mut self) -> Result<&T, StateQueryError>
	where
		T: Default,
	{
		// fetch
		if self.state.is_none() {
			self.state = Some(self.storage.get_value_or_default(&self.state_reference).await?);
		}

		// result
		Ok(self.state.as_ref().unwrap())
	}

	/// Select from state.
	pub async fn map<R, F>(&mut self, f: F) -> Result<StateQuery<S, R>, StateQueryError>
	where
		R: Send + Sync + Clone + DeserializeOwned + 'static,
		F: FnOnce(&T) -> Result<OptionLink<R>, StateQueryError>,
	{
		Ok(StateQuery {
			state: None,
			state_reference: if let Some(state) = self.state().await? { f(state)? } else { OptionLink::none() },
			storage: self.storage.clone(),
		})
	}

	/// Select from state.
	pub async fn then<R, F, Fut>(&mut self, f: F) -> Result<StateQuery<S, R>, StateQueryError>
	where
		R: Send + Sync + Clone + DeserializeOwned + 'static,
		F: FnOnce(&T) -> Fut,
		Fut: Future<Output = Result<OptionLink<R>, StateQueryError>>,
	{
		Ok(StateQuery {
			state: None,
			state_reference: if let Some(state) = self.state().await? { f(state).await? } else { OptionLink::none() },
			storage: self.storage.clone(),
		})
	}

	/// Query from state.
	pub async fn query<R, F, Fut>(self, f: F) -> Result<R, StateQueryError>
	where
		F: FnOnce(StateQuery<S, T>) -> Fut,
		Fut: Future<Output = Result<R, StateQueryError>>,
	{
		Ok(f(self).await?)
	}
}
impl StateQuery<CoStorage, Co> {
	pub async fn from_reducer(reducer: &CoReducer) -> StateQuery<CoStorage, Co> {
		Self { storage: reducer.storage(), state_reference: reducer.co_state().await, state: None }
	}

	pub async fn core<R>(&self, core_name: &str) -> Result<StateQuery<CoStorage, R>, StateQueryError> {
		let core_state_reference = state::core_state_reference(&self.storage, self.state_reference, core_name)
			.await
			.map_err(|err| StateQueryError::NotFound(err.into()))?;
		Ok(StateQuery::<CoStorage, R> {
			storage: self.storage.clone(),
			state_reference: core_state_reference.into(),
			state: None,
		})
	}
}

#[derive(Debug, thiserror::Error)]
pub enum StateQueryError {
	#[error("Query state not found")]
	NotFound(#[source] anyhow::Error),

	#[error("Query storage failed")]
	Storage(#[from] StorageError),
}
impl Into<StorageError> for StateQueryError {
	fn into(self) -> StorageError {
		match &self {
			Self::NotFound(_) => StorageError::Internal(self.into()),
			// copy by keeping StateQueryError in chain
			Self::Storage(storage_err) => match &storage_err {
				StorageError::NotFound(cid, _) => StorageError::NotFound(*cid, self.into()),
				StorageError::Internal(_) => StorageError::Internal(self.into()),
				StorageError::InvalidArgument(_) => StorageError::InvalidArgument(self.into()),
			},
		}
	}
}
