use co_primitives::{cid_to_raw, raw_to_cid, BlockSerializer, RawCid, Storage};
use serde::{de::DeserializeOwned, Serialize};

pub fn write_output_sync<T: Serialize>(storage: &mut impl Storage, value: &T, output: &mut RawCid) {
	let block = BlockSerializer::new().serialize(&value).expect("serialize output");
	let cid = *block.cid();
	storage.set(block);
	*output = cid_to_raw(&cid);
}

pub fn read_input_sync<T: DeserializeOwned>(storage: &impl Storage, input: &RawCid) -> T {
	let input_cid = raw_to_cid(input).expect("valid input CID");
	let input_block = storage.get(&input_cid);
	let reducer_input: T = BlockSerializer::new().deserialize(&input_block).expect("deserialize input");
	reducer_input
}
