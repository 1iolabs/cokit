// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{BlockStorage, StorageError};
use async_trait::async_trait;
use either::Either;

/// Collection which supports transactions.
#[async_trait]
pub trait Transactionable<S>
where
	S: BlockStorage + Clone + 'static,
{
	type Transaction;

	async fn open(&self, storage: &S) -> Result<Self::Transaction, StorageError>;
}

/// Lazy transaction that only opens the transaction when used.
#[derive(Debug)]
pub struct LazyTransaction<S, T>(Either<(S, T), (T::Transaction, bool)>)
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

	/// Whether this transaction has been accessed mutable yet.
	pub fn is_mut_access(&self) -> bool {
		match &self.0 {
			Either::Left(_) => false,
			Either::Right((_, is_mut)) => *is_mut,
		}
	}

	async fn open(&mut self) -> Result<(), StorageError> {
		match &self.0 {
			Either::Left((storage, item)) => {
				self.0 = Either::Right((item.open(storage).await?, false));
			},
			Either::Right(_) => {},
		}
		Ok(())
	}

	pub async fn get(&mut self) -> Result<&T::Transaction, StorageError> {
		self.open().await?;
		Ok(self.opt().expect("initialized after open"))
	}

	pub async fn get_mut(&mut self) -> Result<&mut T::Transaction, StorageError> {
		self.open().await?;
		Ok(self.opt_mut().expect("initialized after open"))
	}

	pub fn opt(&self) -> Option<&T::Transaction> {
		match &self.0 {
			Either::Left(_) => None,
			Either::Right((transaction, _is_mut_access)) => Some(transaction),
		}
	}

	pub fn opt_mut(&mut self) -> Option<&mut T::Transaction> {
		match &mut self.0 {
			Either::Left(_) => None,
			Either::Right((transaction, is_mut_access)) => {
				*is_mut_access = true;
				Some(transaction)
			},
		}
	}

	pub fn opt_if_is_mut_access(&mut self) -> Option<&mut T::Transaction> {
		match &mut self.0 {
			Either::Right((transaction, true)) => Some(transaction),
			_ => None,
		}
	}
}
