use std::collections::HashMap;

use alloc::collections::BTreeMap;
use libipld_cbor::DagCborCodec;
use libipld_core::{ipld::Ipld, codec::Codec, cid::Cid};
use codec::{Input, Output, Encode, Decode, Error};
use scale_info::TypeInfo;
use sp_arithmetic::fixed_point::FixedI64;

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum IpldValue {
    /// Represents the absence of a value or the value undefined.
    Null,
    /// Represents a boolean value.
    Bool(bool),
    /// Represents an integer.
    Integer(i128),
    /// Represents a floating point value.
    /// See: https://stackoverflow.com/a/56564179
    Float(FixedI64), 
    /// Represents an UTF-8 string.
    String(String),
    /// Represents a sequence of bytes.
    Bytes(Vec<u8>),
    /// Represents a list.
    List(Vec<IpldValue>),
    /// Represents a map of strings.
    Map(StorageMap<String, IpldValue>),
    /// Represents a map of integers.
    Link(Cid),
}

impl From<Ipld> for IpldValue {
    fn from(value: Ipld) -> Self {
        
    }
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct IpldValue(Ipld);
// 
// impl TypeInfo for IpldValue {
// 
// }
// 
// impl Encode for IpldValue {
//     fn encode(&self) -> Vec<u8> {
//         DagCborCodec.encode(&self.0).unwrap()
//     }
// }
// 
// impl Decode for IpldValue {
//     fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
//         let data = super::input_read_to_end(input)?;
//         let buf = &data[..];
//         match DagCborCodec.decode(buf) {
//             Result::Ok(value) => Result::Ok(IpldValue(value)),
//             Result::Err(e) => Result::Err(Error::from("Decode failed.")), // todo: propergate error?
//         }
//     }
// }
