use crate::{CoReducer, CoStorage, CO_CORE_NAME_CO};
use anyhow::anyhow;
use async_trait::async_trait;
use co_core_co::Co;
use co_primitives::OptionLink;
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use serde::de::DeserializeOwned;
use std::{future::Future, marker::PhantomData};

#[async_trait]
pub trait Query: Send + Sync {
	type Input: Send + Sync + 'static;
	type Output: Send + Sync + 'static;

	/// Execute the query.
	async fn execute<S>(&mut self, storage: &S, source: Self::Input) -> Result<Self::Output, QueryError>
	where
		S: BlockStorage + 'static;
}

#[async_trait]
pub trait QueryExt: Query {
	/// Execute query using reducer state.
	async fn execute_reducer(&mut self, reducer: &CoReducer) -> Result<(CoStorage, Self::Output), QueryError>
	where
		Self: Query<Input = OptionLink<Co>>,
	{
		let storage = reducer.storage();
		let source = reducer.reducer_state().await.co();
		let result = self.execute(&storage, source).await?;
		Ok((storage, result))
	}

	/// Core.
	///
	/// # Errors
	/// - [`QueryError::NotFound`] - If the core don't exists.
	fn core<T>(self, core_name: &str) -> CoreQuery<'_, Self, T>
	where
		Self: Sized + Query<Output = (OptionLink<Co>, Option<Co>)>,
	{
		CoreQuery::new(self, core_name)
	}

	/// Resolve [`OptionLink<T>`] to [`Option<T>`].
	fn option_link<T>(self) -> OptionLinkQuery<Self, T>
	where
		Self: Sized + Query<Output = OptionLink<T>>,
	{
		OptionLinkQuery::new(self)
	}

	/// Memoize query by return same output for same input.
	fn memoize(self) -> MemoizeQuery<Self>
	where
		Self: Sized,
		Self::Input: Eq + Clone,
		Self::Output: Clone,
	{
		MemoizeQuery::new(self)
	}

	// /// Memoize query by return [`None`] for same input.
	// fn option_memoize<T>(self) -> OptionMemoizeQuery<Self>
	// where
	// 	Self: Sized,
	// 	Self::Input: Eq + Clone,
	// {
	// 	OptionMemoizeQuery::new(self)
	// }

	/// Map
	fn map<T, F>(self, map: F) -> MapQuery<Self, F, T>
	where
		Self: Sized,
		F: Fn(Self::Output) -> T,
	{
		MapQuery::new(self, map)
	}

	/// Async Map
	fn then<T, F, Fut>(self, map: F) -> ThenQuery<Self, F, Fut, T>
	where
		Self: Sized,
		F: Fn(Self::Output) -> Fut,
		Fut: Future<Output = T>,
	{
		ThenQuery::new(self, map)
	}
}
impl<T> QueryExt for T where T: Query {}

pub fn query<T>() -> NewQuery<T> {
	NewQuery { _t: PhantomData }
}

pub fn query_core<'a, T>(core_name: &'a str) -> CoreQuery<'a, OptionLinkQuery<NewQuery<OptionLink<Co>>, Co>, T> {
	query().option_link().core(core_name)
}

// #[async_trait]
// pub trait QueryExecutor {
// 	async fn execute<S, Q>(&mut self, storage: &S, query: &mut Q, source: Q::Input) -> Result<Q::Output, QueryError>
// 	where
// 		S: BlockStorage + 'static,
// 		Q: Query;
// }

// pub struct DefaultQueryExecutor {}
// #[async_trait]
// impl QueryExecutor for DefaultQueryExecutor {
// 	async fn execute<S, Q>(&mut self, storage: &S, query: &mut Q, source: Q::Input) -> Result<Q::Output, QueryError>
// 	where
// 		S: BlockStorage + 'static,
// 		Q: Query,
// 	{
// 		query.execute(storage, source).await
// 	}
// }

pub struct MemoizeQuery<Q>
where
	Q: Query,
	Q::Input: Eq + Clone,
	Q::Output: Clone,
{
	next: Q,
	last: Option<(Q::Input, Q::Output)>,
}
impl<Q> MemoizeQuery<Q>
where
	Q: Query,
	Q::Input: Eq + Clone,
	Q::Output: Clone,
{
	pub fn new(next: Q) -> Self {
		Self { next, last: None }
	}
}
#[async_trait]
impl<Q> Query for MemoizeQuery<Q>
where
	Q: Query,
	Q::Input: Eq + Clone,
	Q::Output: Clone,
{
	type Input = Q::Input;
	type Output = Q::Output;

	async fn execute<S>(&mut self, storage: &S, source: Self::Input) -> Result<Self::Output, QueryError>
	where
		S: BlockStorage + 'static,
	{
		if let Some((last_input, last_output)) = &self.last {
			if last_input == &source {
				return Ok(last_output.clone());
			}
		}
		let last_source = source.clone();
		let result = self.next.execute(storage, source).await?;
		self.last = Some((last_source, result.clone()));
		Ok(result)
	}
}

// pub struct OptionMemoizeQuery<Q>
// where
// 	Q: Query,
// 	Q::Input: Eq + Clone,
// {
// 	next: Q,
// 	last: Option<Q::Input>,
// }
// impl<Q> OptionMemoizeQuery<Q>
// where
// 	Q: Query,
// 	Q::Input: Eq + Clone,
// {
// 	pub fn new(next: Q) -> Self {
// 		Self { next, last: None }
// 	}
// }
// #[async_trait]
// impl<Q> Query for OptionMemoizeQuery<Q>
// where
// 	Q: Query,
// 	Q::Input: Eq + Clone,
// {
// 	type Input = Q::Input;
// 	type Output = Option<Q::Output>;

// 	async fn execute<S>(&mut self, storage: &S, source: Self::Input) -> Result<Self::Output, QueryError>
// 	where
// 		S: BlockStorage + 'static,
// 	{
// 		if let Some(last_input) = &self.last {
// 			if last_input == &source {
// 				return Ok(None);
// 			}
// 		}
// 		let last_source = source.clone();
// 		let result = self.next.execute(storage, source).await?;
// 		self.last = Some(last_source);
// 		Ok(Some(result))
// 	}
// }

pub struct NewQuery<T> {
	_t: PhantomData<T>,
}
#[async_trait]
impl<T> Query for NewQuery<T>
where
	T: Send + Sync + 'static,
{
	type Input = T;
	type Output = T;

	async fn execute<S>(&mut self, _storage: &S, source: Self::Input) -> Result<Self::Output, QueryError>
	where
		S: BlockStorage + 'static,
	{
		Ok(source)
	}
}

/// Resolve OptionLink to actual state.
pub struct OptionLinkQuery<Q, T> {
	query: Q,
	_t: PhantomData<T>,
}
impl<Q, T> OptionLinkQuery<Q, T> {
	pub fn new(query: Q) -> Self {
		Self { query, _t: PhantomData }
	}
}
#[async_trait]
impl<Q, T> Query for OptionLinkQuery<Q, T>
where
	Q: Query<Output = OptionLink<T>>,
	T: DeserializeOwned + Send + Sync + 'static,
{
	type Input = Q::Input;
	type Output = (OptionLink<T>, Option<T>);

	async fn execute<S>(&mut self, storage: &S, source: Self::Input) -> Result<Self::Output, QueryError>
	where
		S: BlockStorage + 'static,
	{
		let link = self.query.execute(storage, source).await?;
		Ok((link, storage.get_value_or_none(&link).await?))
	}
}

/// Map query output to new type.
pub struct MapQuery<Q, F, T> {
	query: Q,
	map: F,
	_t: PhantomData<T>,
}
impl<Q, F, T> MapQuery<Q, F, T> {
	pub fn new(query: Q, map: F) -> Self {
		Self { query, map, _t: PhantomData }
	}
}
#[async_trait]
impl<Q, F, T> Query for MapQuery<Q, F, T>
where
	Q: Query,
	T: Send + Sync + 'static,
	F: Fn(Q::Output) -> T + Send + Sync,
{
	type Input = Q::Input;
	type Output = T;

	async fn execute<S>(&mut self, storage: &S, source: Self::Input) -> Result<Self::Output, QueryError>
	where
		S: BlockStorage + 'static,
	{
		Ok((self.map)(self.query.execute(storage, source).await?))
	}
}

/// Map query output to new type.
pub struct ThenQuery<Q, F, Fut, T> {
	query: Q,
	map: F,
	_fut: PhantomData<Fut>,
	_t: PhantomData<T>,
}
impl<Q, F, Fut, T> ThenQuery<Q, F, Fut, T> {
	pub fn new(query: Q, map: F) -> Self {
		Self { query, map, _t: PhantomData, _fut: PhantomData }
	}
}
#[async_trait]
impl<Q, F, Fut, T> Query for ThenQuery<Q, F, Fut, T>
where
	Q: Query,
	F: Fn(Q::Output) -> Fut + Send + Sync,
	Fut: Future<Output = T> + Send + Sync,
	T: Send + Sync + 'static,
{
	type Input = Q::Input;
	type Output = T;

	async fn execute<S>(&mut self, storage: &S, source: Self::Input) -> Result<Self::Output, QueryError>
	where
		S: BlockStorage + 'static,
	{
		let next = self.query.execute(storage, source).await?;
		Ok((self.map)(next).await)
	}
}

/// Query core.
pub struct CoreQuery<'a, Q, C> {
	query: Q,
	core_name: &'a str,
	_core: PhantomData<C>,
	use_default: bool,
}
impl<'a, Q, C> CoreQuery<'a, Q, C> {
	pub fn new(query: Q, core_name: &'a str) -> Self {
		Self { query, core_name, _core: PhantomData, use_default: false }
	}

	pub fn with_default(mut self) -> Self {
		self.use_default = true;
		self
	}
}
#[async_trait]
impl<'a, Q, C> Query for CoreQuery<'a, Q, C>
where
	C: Default + DeserializeOwned + Clone + Send + Sync + 'static,
	Q: Query<Output = (OptionLink<Co>, Option<Co>)>,
{
	type Input = Q::Input;
	type Output = C;

	async fn execute<S>(&mut self, storage: &S, source: Self::Input) -> Result<Self::Output, QueryError>
	where
		S: BlockStorage + 'static,
	{
		let (co_reference, co) = self.query.execute(storage, source).await?;
		if self.core_name == CO_CORE_NAME_CO {
			Ok(storage.get_default(co_reference.cid()).await?)
		} else if let Some(core) = co.unwrap_or_default().cores.get(self.core_name) {
			if let Some(core_state) = &core.state {
				Ok(storage.get_deserialized(core_state).await?)
			} else {
				Ok(Self::Output::default())
			}
		} else if self.use_default {
			Ok(Self::Output::default())
		} else {
			Err(QueryError::NotFound(anyhow!("Core not found: {}", self.core_name)))
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
	#[error("Query: Not found")]
	NotFound(#[source] anyhow::Error),

	#[error("Query Storage failed")]
	Storage(#[from] StorageError),
}
impl Into<StorageError> for QueryError {
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
