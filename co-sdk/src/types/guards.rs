// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{types::cores::CO_CORE_POA, Cores, CO_CORE_CO};
use co_guard::Guards;
use co_runtime::{Core, GuardReference};

/// Create the default guards registry by scanning built-in cores.
pub fn create_default_guards() -> Guards {
	let mut guards = Guards::new();

	for (name, core) in Cores::default().built_in() {
		if let Some(native_guard) = get_native_opt(&name) {
			if let Core::Wasm(wasm) = core {
				guards.register(name, wasm.to_string());
				guards = guards.with_override(wasm, native_guard);
			}
		}
	}

	guards
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
