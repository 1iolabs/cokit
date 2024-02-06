use co_runtime::Core;
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CO_CORE_CO: &str = "co-core-co";
pub const CO_CORE_KEYSTORE: &str = "co-core-keystore";
pub const CO_CORE_MEMBERSHIP: &str = "co-core-membership";
pub const CO_CORE_ROOM: &str = "co-core-room";

#[derive(Debug, Serialize, Deserialize)]
pub struct Cores {
	cores: HashMap<String, Cid>,
}
impl Default for Cores {
	fn default() -> Self {
		toml::from_str(include_str!("../../../cores/Cores.toml")).unwrap()
	}
}
impl Cores {
	/// Returns the core name used across the co-sdk fot an core create name.
	/// Example: `co-core-co` reutrns `co`
	pub fn to_core_name<'a>(crate_name: &'a str) -> &'a str {
		if crate_name.starts_with("co-core-") {
			return &crate_name["co-core-".len()..];
		}
		crate_name
	}

	/// Get WebAssembly versions CIDs of the built-in cores.
	pub fn built_in(&self) -> HashMap<String, Core> {
		self.cores
			.iter()
			.map(|(name, cid)| (name.to_owned(), Core::Wasm(cid.clone())))
			.collect()
	}

	/// Get native versions of the built-in cores.
	pub fn built_in_native(&self) -> HashMap<String, Core> {
		self.cores.iter().map(|(name, _)| (name.to_owned(), get_native(name))).collect()
	}

	/// Get the binary CID for a built-in core.
	pub fn binary(&self, crate_name: &str) -> Option<Cid> {
		self.cores.get(crate_name).cloned()
	}

	/// Test if the core is a built in core.
	pub fn is_built_in(&self, core: Core) -> bool {
		match &core {
			Core::Native(_) => true,
			Core::Wasm(cid) => self.cores.iter().find(|(_, i)| *i == cid).is_some(),
		}
	}
}

/// Get native core for name.
/// Panics:
/// - When a core listed in Cores.toml are not present in the match below. This can only happen when the list is out of
///   sync.
fn get_native(name: &str) -> Core {
	match name {
		CO_CORE_CO => Core::native::<co_core_co::Co>(),
		CO_CORE_KEYSTORE => Core::native::<co_core_keystore::KeyStore>(),
		CO_CORE_MEMBERSHIP => Core::native::<co_core_membership::Memberships>(),
		CO_CORE_ROOM => Core::native::<co_core_room::Room>(),
		_ => panic!("unknown native core name: {}", name),
	}
}
