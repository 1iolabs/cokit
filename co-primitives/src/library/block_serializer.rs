// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{from_cbor, to_cbor, Block, CborError, KnownMultiCodec, StoreParams};
use cid::Cid;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum BlockSerializerError {
	#[error("Block size {1} exceeds {0}.")]
	BlockToLarge(usize, usize),

	#[error("CBOR failed.")]
	Cbor(#[from] CborError),

	#[error("Deserialize {0} as CBOR failed")]
	CborDeserialize(Cid, #[source] CborError),
}

/// DagCbor Block Serializer/Deserializer.
pub struct BlockSerializer {
	codec: u64,
	max_block_size: Option<usize>,
}
impl BlockSerializer {
	pub fn new() -> Self {
		Self::new_codec(KnownMultiCodec::DagCbor)
	}

	pub fn new_store_params<P: StoreParams>() -> Self {
		Self::new_codec(KnownMultiCodec::DagCbor).with_max_block_size(P::MAX_BLOCK_SIZE)
	}

	pub fn new_codec(codec: impl Into<u64>) -> Self {
		Self { max_block_size: None, codec: codec.into() }
	}

	pub fn with_codec(mut self, codec: impl Into<u64>) -> Self {
		self.codec = codec.into();
		self
	}

	pub fn with_max_block_size(mut self, max_block_size: usize) -> Self {
		self.max_block_size = Some(max_block_size);
		self
	}
}
impl Default for BlockSerializer {
	fn default() -> Self {
		Self::new()
	}
}
impl BlockSerializer {
	/// Serialize item to block.
	pub fn serialize<T>(&self, item: &T) -> Result<Block, BlockSerializerError>
	where
		T: Serialize,
	{
		let data = to_cbor(item)?;
		if let Some(max_block_size) = self.max_block_size {
			if max_block_size < data.len() {
				return Err(BlockSerializerError::BlockToLarge(max_block_size, data.len()));
			}
		}
		Ok(Block::new_data(self.codec, data))
	}

	/// Deserialize block to item.
	pub fn deserialize<'a, T>(&self, item: &'a Block) -> Result<T, BlockSerializerError>
	where
		T: serde::de::Deserialize<'a>,
	{
		// MultiCodec::with_cbor(item.cid())?;
		from_cbor(item.data()).map_err(|err| BlockSerializerError::CborDeserialize(*item.cid(), err))
	}
}

#[cfg(test)]
mod tests {
	use crate::library::block_serializer::BlockSerializer;
	use serde::Serialize;

	#[derive(Debug, Serialize)]
	struct Test {
		hello: String,
	}

	#[test]
	fn should_serialize() {
		let test = Test { hello: "world".to_owned() };
		let block = BlockSerializer::default().serialize(&test).unwrap();
		assert_eq!(block.cid().to_string(), "bafyr4iahzl6dyblh5gjfk5lo46xkkfk7fvxhyot4636rdglz3n5tayegd4");
		assert_eq!(block.data(), [161, 101, 104, 101, 108, 108, 111, 101, 119, 111, 114, 108, 100]);
	}
}
