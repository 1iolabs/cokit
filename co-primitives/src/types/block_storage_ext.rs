use crate::{BlockSerializer, BlockStorage, Link, Linkable, MultiCodec, OptionLink, StorageError};
use cid::Cid;
use either::Either;
use serde::{de::DeserializeOwned, Serialize};

#[allow(async_fn_in_trait)]
pub trait BlockStorageExt: BlockStorage {
	/// Get value from link.
	async fn get_value<T, L>(&self, link: &L) -> Result<T, StorageError>
	where
		T: Send + Sync + Sync + DeserializeOwned,
		L: Linkable<T> + Send + Sync,
	{
		match link.value() {
			Either::Left(cid) => {
				Ok(BlockSerializer::new().deserialize(&self.get(MultiCodec::with_dag_cbor(&cid)?).await?)?)
			},
			Either::Right(value) => Ok(value),
		}
	}

	/// Get value or default from link.
	async fn get_value_or_default<T>(&self, link: &OptionLink<T>) -> Result<T, StorageError>
	where
		T: Send + Sync + DeserializeOwned + Default,
	{
		self.get_default(link.as_ref()).await
	}

	/// Create link for value.
	async fn set_value<T>(&self, value: &T) -> Result<Link<T>, StorageError>
	where
		T: Send + Sync + Serialize,
	{
		let block = BlockSerializer::new()
			.serialize(value)
			.map_err(|e| StorageError::InvalidArgument(e.into()))?;
		Ok(self.set(block).await?.into())
	}

	/// Get deserialized value.
	async fn get_deserialized<T>(&self, item: &Cid) -> Result<T, StorageError>
	where
		T: Send + Sync + DeserializeOwned,
	{
		Ok(BlockSerializer::new()
			.deserialize(&self.get(MultiCodec::with_dag_cbor(item)?).await?)
			.map_err(|e| StorageError::InvalidArgument(e.into()))?)
	}

	/// Set serialized value.
	async fn set_serialized<T>(&self, value: &T) -> Result<Cid, StorageError>
	where
		T: Send + Sync + Serialize,
	{
		let block = BlockSerializer::new().serialize(value)?;
		Ok(self.set(block).await?)
	}

	/// Get deserialized value.
	async fn get_default<T>(&self, item: &Option<Cid>) -> Result<T, StorageError>
	where
		T: Send + Sync + Default + DeserializeOwned,
	{
		Ok(if let Some(item) = item {
			BlockSerializer::new().deserialize(&self.get(MultiCodec::with_dag_cbor(&item)?).await?)?
		} else {
			T::default()
		})
	}
}
impl<T> BlockStorageExt for T where T: BlockStorage + ?Sized {}
