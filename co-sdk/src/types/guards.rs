use crate::{types::cores::CO_CORE_POA, Cores, CO_CORE_CO};
use cid::Cid;
use co_runtime::{Core, GuardReference};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Serialize, Deserialize)]
pub struct Guards {
	guards: HashMap<String, String>,
}
impl Guards {
	/// Returns the core name used across the co-sdk fot an core create name.
	/// Example: `co-core-co` reutrns `co`
	/// See:
	/// - [`CO_CORE_NAME_CO`]
	/// - [`CO_CORE_NAME_KEYSTORE`]
	/// - [`CO_CORE_NAME_MEMBERSHIP`]
	pub fn to_guard_name(crate_name: &str) -> &str {
		Cores::to_core_name(crate_name)
	}

	/// Get WebAssembly versions CIDs of the built-in cores.
	pub fn built_in(&self) -> HashMap<String, GuardReference> {
		self.guards
			.iter()
			.map(|(name, cid)| (name.to_owned(), GuardReference::Wasm(Cid::from_str(cid).expect("valid cid"))))
			.collect()
	}

	/// Get native versions of the built-in cores.
	/// Maps from Crate Name (Cargo.toml) to Core,
	pub fn built_in_native(&self) -> HashMap<String, GuardReference> {
		self.guards.keys().map(|name| (name.to_owned(), get_native(name))).collect()
	}

	/// Map WASM CIDs to native built-in versions.
	pub fn built_in_native_mapping(&self) -> HashMap<Cid, GuardReference> {
		self.guards
			.iter()
			.map(|(name, wasm)| (Cid::from_str(wasm).expect("valid cid"), get_native(name)))
			.collect()
	}

	/// Get the binary CID for a built-in guard.
	pub fn binary(&self, crate_name: &str) -> Option<Cid> {
		self.guards
			.get(crate_name)
			.map(|cid_str| Cid::from_str(cid_str).expect("valid cid"))
	}

	/// Get the GuardReference for a built-in guard.
	pub fn guard(&self, crate_name: &str) -> Option<GuardReference> {
		self.guards.get(crate_name).map(|_cid_str| get_native(crate_name))
	}

	/// Test if the guard is a built in guard.
	pub fn is_built_in(&self, core: GuardReference) -> bool {
		match &core {
			GuardReference::Wasm(cid) => self.guards.iter().any(|(_, i)| &Cid::from_str(i).expect("valid cid") == cid),
			GuardReference::Native(_) => true,
		}
	}
}
impl Default for Guards {
	fn default() -> Self {
		let mut result = Self { guards: Default::default() };

		// we only got buildin guard within the cores (for now) so jsut scan an use them
		for (name, core) in Cores::default().built_in() {
			if let Some(_native_guard) = get_native_opt(&name) {
				if let Core::Wasm(wasm) = core {
					result.guards.insert(name, wasm.to_string());
				}
			}
		}

		result
	}
}

/// Get native guard for name.
fn get_native_opt(name: &str) -> Option<GuardReference> {
	match name {
		CO_CORE_CO => Some(GuardReference::native::<co_core_co::Co>()),
		CO_CORE_POA => Some(GuardReference::native::<co_core_poa::Authority>()),
		_ => None,
	}
}
fn get_native(name: &str) -> GuardReference {
	match get_native_opt(name) {
		Some(i) => i,
		None => panic!("unknown native guard name: {}", name),
	}
}
