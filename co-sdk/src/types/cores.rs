// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_primitives::CoreName;
use co_runtime::Core;
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
pub const CO_CORE_STORAGE: &str = "co-core-storage";
pub const CO_CORE_POA: &str = "co-core-poa";
pub const CO_CORE_BOARD: &str = "co-core-board";
pub const CO_CORE_RICH_TEXT: &str = "co-core-rich-text";

/// CO Core name expected by the SDK implementation (key to `co.cores`).
pub const CO_CORE_NAME_CO: CoreName<'static, co_core_co::Co> = CoreName::new("co");
/// keystore core name expected by the SDK implementation (key to `co.cores`).
pub const CO_CORE_NAME_KEYSTORE: CoreName<'static, co_core_keystore::KeyStore> = CoreName::new("keystore");
/// Membership core names expected by the SDK implementation (key to `co.cores`).
pub const CO_CORE_NAME_MEMBERSHIP: CoreName<'static, co_core_membership::Memberships> = CoreName::new("membership");
pub const CO_CORE_NAME_PIN: CoreName<'static, co_core_pin::Pin> = CoreName::new("pin");
pub const CO_CORE_NAME_STORAGE: CoreName<'static, co_core_storage::Storage> = CoreName::new("storage");

/// Registry for builtin cores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cores {
	/// Built-in cores.
	/// Maps names to Cid strings.
	cores: HashMap<String, String>,

	/// Override core implementations.
	/// This can used to replace known Cid's with trusted native cores.
	#[serde(skip, default)]
	overrides: HashMap<Cid, Core>,
}
impl Default for Cores {
	fn default() -> Self {
		toml::from_str(include_str!("../../../cores/Cores.toml")).unwrap()
	}
}
impl Cores {
	/// Override a core by its [`Cid`].
	pub fn with_override(mut self, core_cid: Cid, core: Core) -> Self {
		self.overrides.insert(core_cid, core);
		self
	}

	/// Returns the core name used across the co-sdk fot an core create name.
	/// Example: `co-core-co` reutrns `co`
	/// See:
	/// - [`CO_CORE_NAME_CO`]
	/// - [`CO_CORE_NAME_KEYSTORE`]
	/// - [`CO_CORE_NAME_MEMBERSHIP`]
	pub fn to_core_name(crate_name: &str) -> &str {
		match crate_name.strip_prefix("co-core-") {
			Some(stripped) => stripped,
			None => crate_name,
		}
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

	/// Map WASM CIDs to (possibly native) built-in versions.
	pub fn mapping(&self) -> HashMap<Cid, Core> {
		let mut result = self.built_in_native_mapping();
		result.extend(self.overrides.clone());
		result
	}

	/// Get the binary CID for a built-in core.
	pub fn binary(&self, crate_name: &str) -> Option<Cid> {
		self.cores
			.get(crate_name)
			.map(|cid_str| Cid::from_str(cid_str).expect("valid cid"))
	}

	/// Get the Core for a built-in core.
	pub fn core(&self, crate_name: &str) -> Option<Core> {
		self.cores.get(crate_name).map(|_cid_str| get_native(crate_name))
	}

	/// Get the Core for a built-in core.
	pub fn built_in_by_name(&self, crate_name: &str) -> Option<(Cid, Core)> {
		self.cores
			.get(crate_name)
			.map(|cid_str| (Cid::from_str(cid_str).expect("valid cid"), get_native(crate_name)))
	}
}

/// Get native core for name.
///
/// # Panics
/// - When a core listed in Cores.toml are not present in the match below. This can only happen when the list is out of
///   sync.
///
/// # Note
/// When changing this you have to run `co core build builtin` to get a new `Cargo.toml`
fn get_native(name: &str) -> Core {
	#[cfg(all(feature = "bundle-wasm-cores", not(clippy)))]
	macro_rules! include_prebuild_core {
		($name:literal) => {
			Core::Binary(
				::zstd::decode_all(::std::io::Cursor::new(include_bytes!(concat!(
					"../../../target-wasm/wasm32-unknown-unknown/release/co_core_",
					$name,
					".wasm.zst"
				))))
				.expect("to decode"),
			)
		};
	}
	#[cfg(all(feature = "bundle-wasm-cores", clippy))]
	macro_rules! include_prebuild_core {
		($name:literal) => {
			Core::Binary(Vec::new())
		};
	}
	#[cfg(feature = "bundle-wasm-cores")]
	match name {
		CO_CORE_CO => include_prebuild_core!("co"),
		CO_CORE_FILE => include_prebuild_core!("file"),
		CO_CORE_KEYSTORE => include_prebuild_core!("keystore"),
		CO_CORE_MEMBERSHIP => include_prebuild_core!("membership"),
		CO_CORE_PIN => include_prebuild_core!("pin"),
		CO_CORE_ROOM => include_prebuild_core!("room"),
		CO_CORE_ROLE => include_prebuild_core!("role"),
		CO_CORE_DATA_SERIES => include_prebuild_core!("data_series"),
		CO_CORE_STORAGE => include_prebuild_core!("storage"),
		CO_CORE_POA => include_prebuild_core!("poa"),
		CO_CORE_BOARD => include_prebuild_core!("board"),
		CO_CORE_RICH_TEXT => include_prebuild_core!("rich_text"),
		_ => panic!("unknown native core name: {}", name),
	}
	#[cfg(not(feature = "bundle-wasm-cores"))]
	match name {
		CO_CORE_CO => Core::native_async::<co_core_co::Co, co_core_co::CoAction>(),
		CO_CORE_FILE => Core::native::<co_core_file::File>(),
		CO_CORE_KEYSTORE => Core::native_async::<co_core_keystore::KeyStore, co_core_keystore::KeyStoreAction>(),
		CO_CORE_MEMBERSHIP => Core::native_async::<co_core_membership::Memberships, _>(),
		CO_CORE_PIN => Core::native::<co_core_pin::Pin>(),
		CO_CORE_ROOM => Core::native::<co_core_room::Room>(),
		CO_CORE_ROLE => Core::native::<co_core_role::Roles>(),
		CO_CORE_DATA_SERIES => Core::native::<co_core_data_series::DataSeries>(),
		CO_CORE_STORAGE => Core::native_async::<co_core_storage::Storage, co_core_storage::StorageAction>(),
		CO_CORE_POA => Core::native_async::<co_core_poa::Authority, co_core_poa::AuthorityAction>(),
		CO_CORE_BOARD => Core::native_async::<co_core_board::Board, co_core_board::BoardAction>(),
		CO_CORE_RICH_TEXT => Core::native_async::<co_core_rich_text::RichText, co_core_rich_text::RichTextAction>(),
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
