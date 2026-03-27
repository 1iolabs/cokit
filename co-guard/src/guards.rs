// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_runtime::GuardReference;
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
impl Default for Guards {
	fn default() -> Self {
		Self::new()
	}
}
impl Guards {
	pub fn new() -> Self {
		Self { guards: Default::default(), overrides: Default::default() }
	}

	/// Register a guard by name and CID string.
	pub fn register(&mut self, name: String, cid_str: String) {
		self.guards.insert(name, cid_str);
	}

	/// Override a guard by its [`Cid`].
	pub fn with_override(mut self, guard_cid: Cid, guard: GuardReference) -> Self {
		self.overrides.insert(guard_cid, guard);
		self
	}

	/// Strips the `co-core-` prefix from a crate name.
	/// Example: `co-core-co` returns `co`
	pub fn to_guard_name(crate_name: &str) -> &str {
		match crate_name.strip_prefix("co-core-") {
			Some(stripped) => stripped,
			None => crate_name,
		}
	}

	/// Get WebAssembly versions CIDs of the built-in guards.
	pub fn built_in(&self) -> HashMap<String, GuardReference> {
		self.guards
			.iter()
			.map(|(name, cid)| (name.to_owned(), GuardReference::Wasm(Cid::from_str(cid).expect("valid cid"))))
			.collect()
	}

	/// Map WASM CIDs to (possibly native) built-in versions.
	/// Uses overrides registered via [`with_override`].
	pub fn mapping(&self) -> HashMap<Cid, GuardReference> {
		self.overrides.clone()
	}

	/// Get the binary CID for a built-in guard.
	pub fn binary(&self, crate_name: &str) -> Option<Cid> {
		self.guards
			.get(crate_name)
			.map(|cid_str| Cid::from_str(cid_str).expect("valid cid"))
	}

	/// Get the built-in guard override by crate name.
	pub fn guard(&self, crate_name: &str) -> Option<GuardReference> {
		let cid = self.binary(crate_name)?;
		self.overrides.get(&cid).cloned()
	}

	/// Get the CID and GuardReference for a built-in guard by crate name.
	pub fn built_in_by_name(&self, crate_name: &str) -> Option<(Cid, GuardReference)> {
		let cid = self.binary(crate_name)?;
		let guard = self.overrides.get(&cid).cloned()?;
		Some((cid, guard))
	}
}
