use crate::Storage;
use co_primitives::{BlockSerializer, Link, Linkable, MultiCodec};
use either::Either;
use std::convert::Infallible;

pub trait StorageExt: Storage {
	/// Get value from link.
	fn get_value<T, L: Linkable<T>>(&self, link: &L) -> Result<T, StorageError>
	where
		T: Clone + serde::de::DeserializeOwned,
	{
		match link.value() {
			Either::Left(cid) => Ok(BlockSerializer::new()
				.deserialize(
					&self.get(MultiCodec::dag_cbor(&cid).map_err(|e| StorageError::InvalidArgument(e.into()))?),
				)
				.map_err(|e| StorageError::InvalidArgument(e.into()))?),
			Either::Right(value) => Ok(value),
		}
	}

	/// Create link for value.
	fn set_value<T>(&mut self, value: &T) -> Link<T>
	where
		T: Clone + serde::Serialize,
	{
		self.set(BlockSerializer::new().serialize(value).expect("value to serialize"))
			.into()
	}
}
impl<T> StorageExt for T where T: Storage + ?Sized {}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
	#[error("Invalid argument")]
	InvalidArgument(#[source] anyhow::Error),
}
impl From<serde_ipld_dagcbor::DecodeError<Infallible>> for StorageError {
	fn from(value: serde_ipld_dagcbor::DecodeError<Infallible>) -> Self {
		StorageError::InvalidArgument(value.into())
	}
}
