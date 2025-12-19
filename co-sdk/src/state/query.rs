use crate::{CoReducer, CoStorage, CO_CORE_NAME_CO};
use anyhow::anyhow;
use async_trait::async_trait;
use co_core_co::Co;
use co_primitives::{CoMap, CoreName, OptionLink, Transactionable};
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use serde::{de::DeserializeOwned, Serialize};
use std::{future::Future, hash::Hash, marker::PhantomData};

#[async_trait]
pub trait Query: Send {
	type Storage: BlockStorage + Clone + 'static;
	type Input: Send + 'static;
	type Output: Send + 'static;

	/// Execute the query.
	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError>;
}

#[async_trait]
pub trait QueryExt: Query {
	/// Execute query using reducer state.
	async fn execute_reducer(&mut self, reducer: &CoReducer) -> Result<(CoStorage, Self::Output), QueryError>
	where
		Self: Query<Storage = CoStorage, Input = OptionLink<Co>>,
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
	fn core<'a, T>(self, core_name: CoreName<'a, T>) -> CoreQuery<'a, Self, T>
	where
		Self: Sized + Query<Output = (OptionLink<Co>, Option<Co>)>,
	{
		CoreQuery::new(self, core_name.name())
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
		F: Fn(&Self::Storage, Self::Output) -> Fut,
		Fut: Future<Output = T>,
	{
		ThenQuery::new(self, map)
	}

	/// Open transaction.
	fn open(self) -> OpenQuery<Self>
	where
		Self: Sized,
	{
		OpenQuery { inner: self }
	}

	/// Query map value by its key.
	fn get_value<K, V>(self, key: K) -> CoMapGetQuery<Self, K, V>
	where
		K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
		V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
		Self: Sized + Query<Output = CoMap<K, V>>,
	{
		CoMapGetQuery::new(self, key)
	}

	/// Query default.
	fn with_default<T>(self) -> DefaultQuery<Self, T>
	where
		Self: Sized + Query<Output = Option<T>>,
		T: Default + DeserializeOwned + Send + Sync + 'static,
	{
		DefaultQuery::new(self)
	}
}
impl<T> QueryExt for T where T: Query {}

pub fn query<S, T>() -> NewQuery<S, T> {
	NewQuery { _s: PhantomData, _t: PhantomData }
}

pub fn query_core<'a, S, T>(
	core_name: CoreName<'a, T>,
) -> CoreQuery<'a, OptionLinkQuery<NewQuery<S, OptionLink<Co>>, Co>, T>
where
	S: BlockStorage + Clone + 'static,
{
	query().option_link().core(core_name)
}

pub struct OpenQuery<Q> {
	inner: Q,
}
impl<Q> OpenQuery<Q>
where
	Q: Query,
	Q::Output: Transactionable<Q::Storage>,
	<Q::Output as Transactionable<Q::Storage>>::Transaction: Send + 'static,
{
	pub fn new(inner: Q) -> Self {
		Self { inner }
	}
}
#[async_trait]
impl<Q> Query for OpenQuery<Q>
where
	Q: Query,
	Q::Output: Transactionable<Q::Storage>,
	<Q::Output as Transactionable<Q::Storage>>::Transaction: Send + 'static,
{
	type Storage = Q::Storage;
	type Input = Q::Input;
	type Output = <Q::Output as Transactionable<Q::Storage>>::Transaction;

	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError> {
		let input = self.inner.execute(storage, source).await?;
		Ok(input.open(storage).await?)
	}
}

pub struct CoMapGetQuery<Q, K, V> {
	inner: Q,
	key: K,
	value: PhantomData<V>,
}
impl<Q, K, V> CoMapGetQuery<Q, K, V>
where
	Q: Query,
{
	pub fn new(inner: Q, key: K) -> Self {
		Self { inner, key, value: PhantomData }
	}
}
#[async_trait]
impl<Q, K, V> Query for CoMapGetQuery<Q, K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	Q: Query<Output = CoMap<K, V>>,
{
	type Storage = Q::Storage;
	type Input = Q::Input;
	type Output = Option<V>;

	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError> {
		let input = self.inner.execute(storage, source).await?;
		Ok(input.get(storage, &self.key).await?)
	}
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
	type Storage = Q::Storage;
	type Input = Q::Input;
	type Output = Q::Output;

	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError> {
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

// 	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError>
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

pub struct NewQuery<S, T> {
	_s: PhantomData<S>,
	_t: PhantomData<T>,
}
#[async_trait]
impl<S, T> Query for NewQuery<S, T>
where
	S: BlockStorage + Clone + 'static,
	T: Send + 'static,
{
	type Storage = S;
	type Input = T;
	type Output = T;

	async fn execute(&mut self, _storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError> {
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
	type Storage = Q::Storage;
	type Input = Q::Input;
	type Output = (OptionLink<T>, Option<T>);

	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError> {
		let link = self.query.execute(storage, source).await?;
		Ok((link, storage.get_value_or_none(&link).await?))
	}
}

/// Use default if value is None.
pub struct DefaultQuery<Q, T> {
	query: Q,
	_t: PhantomData<T>,
}
impl<Q, T> DefaultQuery<Q, T> {
	pub fn new(query: Q) -> Self {
		Self { query, _t: PhantomData }
	}
}
#[async_trait]
impl<Q, T> Query for DefaultQuery<Q, T>
where
	Q: Query<Output = Option<T>>,
	T: Default + DeserializeOwned + Send + Sync + 'static,
{
	type Storage = Q::Storage;
	type Input = Q::Input;
	type Output = T;

	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError> {
		Ok(self.query.execute(storage, source).await?.unwrap_or_default())
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
	type Storage = Q::Storage;
	type Input = Q::Input;
	type Output = T;

	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError> {
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
	type Storage = Q::Storage;
	type Input = Q::Input;
	type Output = T;

	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError> {
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
	type Storage = Q::Storage;
	type Input = Q::Input;
	type Output = C;

	async fn execute(&mut self, storage: &Self::Storage, source: Self::Input) -> Result<Self::Output, QueryError> {
		let (co_reference, co) = self.query.execute(storage, source).await?;
		if CO_CORE_NAME_CO == self.core_name {
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
impl From<QueryError> for StorageError {
	fn from(value: QueryError) -> Self {
		match &value {
			QueryError::NotFound(_) => StorageError::Internal(value.into()),
			// copy by keeping StateQueryError in chain
			QueryError::Storage(storage_err) => match &storage_err {
				StorageError::NotFound(cid, _) => StorageError::NotFound(*cid, value.into()),
				StorageError::Internal(_) => StorageError::Internal(value.into()),
				StorageError::InvalidArgument(_) => StorageError::InvalidArgument(value.into()),
			},
		}
	}
}
