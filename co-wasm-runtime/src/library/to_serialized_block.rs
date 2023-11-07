use libipld::{
	cbor::DagCborCodec,
	multihash::{Code, MultihashDigest},
	Block, Cid, DefaultParams,
};
use serde::Serialize;
use serde_ipld_dagcbor::EncodeError;
use std::collections::TryReserveError;

#[derive(Debug, Clone)]
pub struct SerializeOptions {
	pub codec: u64,
}
impl Default for SerializeOptions {
	fn default() -> Self {
		Self { codec: DagCborCodec.into() }
	}
}

pub fn to_serialized_block<T>(
	item: &T,
	options: SerializeOptions,
) -> Result<Block<DefaultParams>, EncodeError<TryReserveError>>
where
	T: Serialize,
{
	let data = serde_ipld_dagcbor::to_vec(item)?;
	let mh = Code::Blake3_256.digest(&data);
	let cid = Cid::new_v1(options.codec, mh);
	Ok(Block::new_unchecked(cid, data))
}

#[cfg(test)]
mod tests {
	use crate::library::to_serialized_block::to_serialized_block;
	use serde::Serialize;

	#[derive(Debug, Serialize)]
	struct Test {
		hello: String,
	}

	#[test]
	fn should_serialize() {
		let test = Test { hello: "world".to_owned() };
		let block = to_serialized_block(&test, Default::default()).unwrap();
		assert_eq!("bafyr4iahzl6dyblh5gjfk5lo46xkkfk7fvxhyot4636rdglz3n5tayegd4", block.cid().to_string());
		assert_eq!([161, 101, 104, 101, 108, 108, 111, 101, 119, 111, 114, 108, 100], block.data());
	}
}
