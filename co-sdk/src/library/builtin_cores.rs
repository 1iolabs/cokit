use crate::Cores;
use cid::Cid;
use std::collections::BTreeSet;

/// Get a set of built-in cores.
pub fn builtin_cores() -> BTreeSet<Cid> {
	let builtin_cores: BTreeSet<Cid> = Cores::default().built_in_native_mapping().into_keys().collect();
	builtin_cores
}
