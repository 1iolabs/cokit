// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::Storage;
use co_primitives::{BlockSerializer, Link, Linkable, MultiCodec, StorageError};
use either::Either;

pub trait StorageExt: Storage {
	/// Get value from link.
	fn get_value<T, L: Linkable<T>>(&self, link: &L) -> Result<T, StorageError>
	where
		T: Clone + serde::de::DeserializeOwned,
	{
		match link.value() {
			Either::Left(cid) => Ok(BlockSerializer::new()
				.deserialize(
					&self.get(MultiCodec::with_cbor(&cid).map_err(|e| StorageError::InvalidArgument(e.into()))?),
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
