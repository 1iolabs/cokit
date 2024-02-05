use crate::{BlockStorage, StorageError};
use async_trait::async_trait;
use co_primitives::{BlockSerializer, Link, Linkable};
use libipld::{cbor::DagCborCodec, Cid};

#[async_trait]
pub trait BlockStorageExt: BlockStorage + Send + Sync + 'static {
	/// Get value from link.
	async fn get_value<T, L>(&self, link: &L) -> Result<T, StorageError>
	where
		T: Send + Sync + Clone + serde::de::DeserializeOwned,
		L: Linkable<T> + Send + Sync,
	{
		match link.cid().codec() {
			v if v == Into::<u64>::into(DagCborCodec) => Ok(BlockSerializer::new()
				.deserialize(&self.get(link.cid()).await?)
				.map_err(|e| StorageError::InvalidArgument(e.into()))?),
			v => Err(StorageError::InvalidArgument(anyhow::anyhow!("unknown codec {}", v))),
		}
	}

	/// Create link for value.
	async fn set_value<T>(&mut self, value: &T) -> Result<Link<T>, StorageError>
	where
		T: Send + Sync + Clone + serde::Serialize,
	{
		let block = BlockSerializer::new()
			.serialize(value)
			.map_err(|e| StorageError::InvalidArgument(e.into()))?;
		Ok(self.set(block).await?.into())
	}

	/// Get deserialized value.
	async fn get_deserialized<T>(&self, item: &Cid) -> Result<T, StorageError>
	where
		T: Send + Sync + Clone + serde::de::DeserializeOwned,
	{
		match item.codec() {
			v if v == Into::<u64>::into(DagCborCodec) => Ok(BlockSerializer::new()
				.deserialize(&self.get(item).await?)
				.map_err(|e| StorageError::InvalidArgument(e.into()))?),
			v => Err(StorageError::InvalidArgument(anyhow::anyhow!("unknown codec {}", v))),
		}
	}

	/// Set serialized value.
	async fn set_serialized<T>(&mut self, value: &T) -> Result<Cid, StorageError>
	where
		T: Send + Sync + Clone + serde::Serialize,
	{
		let block = BlockSerializer::new()
			.serialize(value)
			.map_err(|e| StorageError::InvalidArgument(e.into()))?;
		Ok(self.set(block).await?)
	}
}
impl<T> BlockStorageExt for T where T: BlockStorage + ?Sized + Send + Sync + 'static {}
