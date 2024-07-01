use co_runtime::Core;
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

pub const CO_CORE_CO: &str = "co-core-co";
pub const CO_CORE_FILE: &str = "co-core-file";
pub const CO_CORE_KEYSTORE: &str = "co-core-keystore";
pub const CO_CORE_MEMBERSHIP: &str = "co-core-membership";
pub const CO_CORE_PIN: &str = "co-core-pin";
pub const CO_CORE_ROOM: &str = "co-core-room";
pub const CO_CORE_ROLE: &str = "co-core-role";
pub const CO_CORE_DATA_SERIES: &str = "co-core-data-series";

/// CO Core name expected by the SDK implementation (key to `co.cores`).
pub const CO_CORE_NAME_CO: &str = "co";
/// keystore core name expected by the SDK implementation (key to `co.cores`).
pub const CO_CORE_NAME_KEYSTORE: &str = "keystore";
/// Membership core names expected by the SDK implementation (key to `co.cores`).
pub const CO_CORE_NAME_MEMBERSHIP: &str = "membership";
pub const CO_CORE_NAME_PIN: &str = "pin";

#[derive(Debug, Serialize, Deserialize)]
pub struct Cores {
	cores: HashMap<String, String>,
}
impl Default for Cores {
	fn default() -> Self {
		toml::from_str(include_str!("../../../cores/Cores.toml")).unwrap()
	}
}
impl Cores {
	/// Returns the core name used across the co-sdk fot an core create name.
	/// Example: `co-core-co` reutrns `co`
	/// See:
	/// - [`CO_CORE_NAME_CO`]
	/// - [`CO_CORE_NAME_KEYSTORE`]
	/// - [`CO_CORE_NAME_MEMBERSHIP`]
	pub fn to_core_name(crate_name: &str) -> &str {
		if crate_name.starts_with("co-core-") {
			return &crate_name["co-core-".len()..];
		}
		crate_name
	}

	/// Get WebAssembly versions CIDs of the built-in cores.
	pub fn built_in(&self) -> HashMap<String, Core> {
		self.cores
			.iter()
			.map(|(name, cid)| (name.to_owned(), Core::Wasm(Cid::from_str(cid).expect("valid cid"))))
			.collect()
	}

	/// Get native versions of the built-in cores.
	/// Maps from Crate Name (Cargo.toml) to Core,
	pub fn built_in_native(&self) -> HashMap<String, Core> {
		self.cores.keys().map(|name| (name.to_owned(), get_native(name))).collect()
	}

	/// Map WASM CIDs to native built-in versions.
	pub fn built_in_native_mapping(&self) -> HashMap<Cid, Core> {
		self.cores
			.iter()
			.map(|(name, wasm)| (Cid::from_str(wasm).expect("valid cid"), get_native(name)))
			.collect()
	}

	/// Get the binary CID for a built-in core.
	pub fn binary(&self, crate_name: &str) -> Option<Cid> {
		self.cores
			.get(crate_name)
			.map(|cid_str| Cid::from_str(cid_str).expect("valid cid"))
	}

	/// Test if the core is a built in core.
	pub fn is_built_in(&self, core: Core) -> bool {
		match &core {
			Core::Native(_) => true,
			Core::Wasm(cid) => self.cores.iter().any(|(_, i)| &Cid::from_str(i).expect("valid cid") == cid),
		}
	}
}

/// Get native core for name.
/// Panics:
/// - When a core listed in Cores.toml are not present in the match below. This can only happen when the list is out of
///   sync.
/// Note: When changing this you have to run `co core build builtin` to get ``
fn get_native(name: &str) -> Core {
	match name {
		CO_CORE_CO => Core::native::<co_core_co::Co>(),
		CO_CORE_FILE => Core::native::<co_core_file::File>(),
		CO_CORE_KEYSTORE => Core::native::<co_core_keystore::KeyStore>(),
		CO_CORE_MEMBERSHIP => Core::native::<co_core_membership::Memberships>(),
		CO_CORE_PIN => Core::native::<co_core_pin::Pin>(),
		CO_CORE_ROOM => Core::native::<co_core_room::Room>(),
		CO_CORE_ROLE => Core::native::<co_core_role::Roles>(),
		CO_CORE_DATA_SERIES => Core::native::<co_core_data_series::DataSeries>(),
		_ => panic!("unknown native core name: {}", name),
	}
}

#[cfg(test)]
mod tests {
	use crate::Cores;

	#[test]
	fn test_built_in_native() {
		// make sure all cores are registered as native
		assert_eq!(Cores::default().built_in().len(), Cores::default().built_in_native().len());
	}
}
