use crate::{BlockStorage, StorageError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{BlockSerializer, Link, Linkable, MultiCodec};
use either::Either;

#[async_trait]
pub trait BlockStorageExt: BlockStorage + Send + Sync + 'static {
	/// Get value from link.
	async fn get_value<T, L>(&self, link: &L) -> Result<T, StorageError>
	where
		T: Send + Sync + serde::de::DeserializeOwned,
		L: Linkable<T> + Send + Sync,
	{
		match link.value() {
			Either::Left(cid) => Ok(BlockSerializer::new()
				.deserialize(&self.get(MultiCodec::with_dag_cbor(&cid)?).await?)
				.map_err(|e| StorageError::InvalidArgument(e.into()))?),
			Either::Right(value) => Ok(value),
		}
	}

	/// Create link for value.
	async fn set_value<T>(&self, value: &T) -> Result<Link<T>, StorageError>
	where
		T: Send + Sync + serde::Serialize,
	{
		let block = BlockSerializer::new()
			.serialize(value)
			.map_err(|e| StorageError::InvalidArgument(e.into()))?;
		Ok(self.set(block).await?.into())
	}

	/// Get deserialized value.
	async fn get_deserialized<T>(&self, item: &Cid) -> Result<T, StorageError>
	where
		T: Send + Sync + serde::de::DeserializeOwned,
	{
		Ok(BlockSerializer::new()
			.deserialize(&self.get(MultiCodec::with_dag_cbor(item)?).await?)
			.map_err(|e| StorageError::InvalidArgument(e.into()))?)
	}

	/// Set serialized value.
	async fn set_serialized<T>(&self, value: &T) -> Result<Cid, StorageError>
	where
		T: Send + Sync + serde::Serialize,
	{
		let block = BlockSerializer::new()
			.serialize(value)
			.map_err(|e| StorageError::InvalidArgument(e.into()))?;
		Ok(self.set(block).await?)
	}

	/// Get deserialized value.
	async fn get_default<T>(&self, item: &Option<Cid>) -> Result<T, StorageError>
	where
		T: Send + Default + Sync + serde::de::DeserializeOwned,
	{
		Ok(if let Some(item) = item {
			BlockSerializer::new()
				.deserialize(&self.get(MultiCodec::with_dag_cbor(item)?).await?)
				.map_err(|e| StorageError::InvalidArgument(e.into()))?
		} else {
			T::default()
		})
	}
}
impl<T> BlockStorageExt for T where T: BlockStorage + ?Sized + Send + Sync + 'static {}
