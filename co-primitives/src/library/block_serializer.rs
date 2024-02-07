use libipld::{
	cbor::DagCborCodec,
	multihash::{Code, MultihashDigest},
	store::StoreParams,
	Block, Cid, DefaultParams,
};
use serde::Serialize;
use serde_ipld_dagcbor::{DecodeError, EncodeError};
use std::{any::type_name, collections::TryReserveError, convert::Infallible, marker::PhantomData};

#[derive(Debug, thiserror::Error)]
pub enum BlockSerializerError {
	#[error("Block size {1} exceeds {0}.")]
	BlockToLarge(usize, usize),

	#[error("Encode failed.")]
	Encode(#[from] EncodeError<TryReserveError>),

	#[error("Decode {0:?} to '{1}' failed")]
	Decode(Cid, String, DecodeError<Infallible>),
}

/// DagCbor Block Serializer/Deserializer.
pub struct BlockSerializer<S> {
	_s: PhantomData<S>,
	codec: u64,
}
impl<S> BlockSerializer<S> {
	pub fn new() -> Self {
		Self::new_codec(DagCborCodec.into())
	}

	pub fn new_codec(codec: u64) -> Self {
		Self { _s: Default::default(), codec }
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
		let data = serde_ipld_dagcbor::to_vec(item)?;
		if S::MAX_BLOCK_SIZE < data.len() {
			return Err(BlockSerializerError::BlockToLarge(S::MAX_BLOCK_SIZE, data.len()))
		}
		let mh = Code::Blake3_256.digest(&data);
		let cid = Cid::new_v1(self.codec, mh);
		Ok(Block::new_unchecked(cid, data))
	}

	/// Deserialize block to item.
	pub fn deserialize<'a, T>(&self, item: &'a Block<S>) -> Result<T, BlockSerializerError>
	where
		T: serde::de::Deserialize<'a>,
	{
		Ok(serde_ipld_dagcbor::from_slice::<'a, T>(item.data())
			.map_err(|e| BlockSerializerError::Decode(item.cid().clone(), type_name::<T>().to_owned(), e.into()))?)
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
