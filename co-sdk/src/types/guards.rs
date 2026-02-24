// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{types::cores::CO_CORE_POA, Cores, CO_CORE_CO};
use cid::Cid;
use co_runtime::{Core, GuardReference};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

/// Registry for builtin guards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guards {
	guards: HashMap<String, String>,

	/// Override core implementations.
	/// This can used to replace known Cid's with trusted native cores.
	#[serde(skip, default)]
	overrides: HashMap<Cid, GuardReference>,
}
impl Guards {
	/// Override a core by its [`Cid`].
	pub fn with_override(mut self, guard_cid: Cid, guard: GuardReference) -> Self {
		self.overrides.insert(guard_cid, guard);
		self
	}

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

	/// Map WASM CIDs to (possibly native) built-in versions.
	pub fn mapping(&self) -> HashMap<Cid, GuardReference> {
		let mut result = self.built_in_native_mapping();
		result.extend(self.overrides.clone());
		result
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

	/// Get the GuardReference for a built-in guard.
	pub fn built_in_by_name(&self, crate_name: &str) -> Option<(Cid, GuardReference)> {
		self.guards
			.get(crate_name)
			.map(|cid_str| (Cid::from_str(cid_str).expect("valid cid"), get_native(crate_name)))
	}
}
impl Default for Guards {
	fn default() -> Self {
		let mut result = Self { guards: Default::default(), overrides: Default::default() };

		// we only got buildin guard within the cores (for now) so just scan an use them
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
	#[cfg(feature = "bundle-wasm-cores")]
	match name {
		CO_CORE_CO => Some(get_from_core(name)),
		CO_CORE_POA => Some(get_from_core(name)),
		_ => None,
	}

	#[cfg(not(feature = "bundle-wasm-cores"))]
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

#[cfg(feature = "bundle-wasm-cores")]
fn get_from_core(name: &str) -> GuardReference {
	let (_cid, core) = Cores::default()
		.built_in_by_name(name)
		.ok_or(anyhow::anyhow!("unknown native guard name: {}", name))
		.expect("buildin core");
	match core {
		Core::Wasm(cid) => GuardReference::Wasm(cid),
		Core::Binary(binary) => GuardReference::Binary(binary),
		_ => panic!("native is not allowed with bundle-wasm-cores"),
	}
}
