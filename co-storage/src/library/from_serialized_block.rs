use libipld::{Block, DefaultParams};
use serde_ipld_dagcbor::DecodeError;
use std::convert::Infallible;

pub fn from_serialized_block<'a, T>(item: &'a Block<DefaultParams>) -> Result<T, DecodeError<Infallible>>
where
	T: serde::de::Deserialize<'a>,
{
	serde_ipld_dagcbor::from_slice::<'a, T>(item.data())
}
