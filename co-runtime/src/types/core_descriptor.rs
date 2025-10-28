use cid::Cid;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreDescriptor {
	/// Reference to the WASM binary.
	#[serde(rename = "w")]
	pub wasm: Cid,

	/// Reference to additional native binaries.
	#[serde(rename = "n", default, skip_serializing_if = "BTreeMap::is_empty")]
	pub native: BTreeMap<String, Cid>,
}

// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
// pub struct CoreTarget(CoreArchitecture, CoreVendor);

// #[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq, Eq, PartialOrd, Ord)]
// #[repr(u8)]
// pub enum CoreVendor {
// 	Unknown = 0,
// 	Apple = 1,
// }

// #[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq, Eq, PartialOrd, Ord)]
// #[repr(u8)]
// pub enum CoreArchitecture {
// 	X86_64 = 7u8 | 0x02u8,
// 	Arm64 = 12u8 | 0x02u8,
// }
