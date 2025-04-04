use crate::{from_cbor, to_cbor, Block, CborError, DefaultParams, KnownMultiCodec, StoreParams};
use serde::Serialize;
use std::marker::PhantomData;

#[derive(Debug, thiserror::Error)]
pub enum BlockSerializerError {
	#[error("Block size {1} exceeds {0}.")]
	BlockToLarge(usize, usize),

	#[error("CBOR failed.")]
	Cbor(#[from] CborError),
}

/// DagCbor Block Serializer/Deserializer.
pub struct BlockSerializer<S> {
	_s: PhantomData<S>,
	codec: u64,
}
impl<S> BlockSerializer<S> {
	pub fn new() -> Self {
		Self::new_codec(KnownMultiCodec::DagCbor)
	}

	pub fn new_codec(codec: impl Into<u64>) -> Self {
		Self { _s: Default::default(), codec: codec.into() }
	}

	pub fn with_codec(mut self, codec: impl Into<u64>) -> Self {
		self.codec = codec.into();
		self
	}
}
impl Default for BlockSerializer<DefaultParams> {
	fn default() -> Self {
		Self::new()
	}
}
impl<S> BlockSerializer<S>
where
	S: StoreParams,
{
	/// Serialize item to block.
	pub fn serialize<T>(&self, item: &T) -> Result<Block<S>, BlockSerializerError>
	where
		T: Serialize,
	{
		let data = to_cbor(item)?;
		if S::MAX_BLOCK_SIZE < data.len() {
			return Err(BlockSerializerError::BlockToLarge(S::MAX_BLOCK_SIZE, data.len()));
		}
		Ok(Block::new_data(self.codec, data))
	}

	/// Deserialize block to item.
	pub fn deserialize<'a, T>(&self, item: &'a Block<S>) -> Result<T, BlockSerializerError>
	where
		T: serde::de::Deserialize<'a>,
	{
		// MultiCodec::with_cbor(item.cid())?;
		Ok(from_cbor(item.data())?)
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
