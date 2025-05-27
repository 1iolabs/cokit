use crate::{BlockStorage, StorageError};
use async_trait::async_trait;
use either::Either;

#[async_trait]
pub trait Transactionable<S>
where
	S: BlockStorage + Clone + 'static,
{
	type Transaction;

	async fn open(&self, storage: &S) -> Result<Self::Transaction, StorageError>;
}

#[derive(Debug)]
pub struct LazyTransaction<S, T>(Either<(S, T), T::Transaction>)
where
	S: BlockStorage + Clone + 'static,
	T: Transactionable<S> + 'static;

impl<S, T> LazyTransaction<S, T>
where
	S: BlockStorage + Clone + 'static,
	T: Transactionable<S> + 'static,
{
	pub fn new(storage: S, init: T) -> Self {
		Self(Either::Left((storage, init)))
	}

	async fn open(&mut self) -> Result<(), StorageError> {
		match &self.0 {
			Either::Left((storage, item)) => {
				self.0 = Either::Right(item.open(&storage).await?);
			},
			Either::Right(_) => {},
		}
		Ok(())
	}

	pub async fn get(&mut self) -> Result<&T::Transaction, StorageError> {
		self.open().await?;
		Ok(self.opt().unwrap())
	}

	pub async fn get_mut(&mut self) -> Result<&mut T::Transaction, StorageError> {
		self.open().await?;
		Ok(self.opt_mut().unwrap())
	}

	pub fn opt(&self) -> Option<&T::Transaction> {
		match &self.0 {
			Either::Left(_) => None,
			Either::Right(transaction) => Some(transaction),
		}
	}

	pub fn opt_mut(&mut self) -> Option<&mut T::Transaction> {
		match &mut self.0 {
			Either::Left(_) => None,
			Either::Right(transaction) => Some(transaction),
		}
	}
}
