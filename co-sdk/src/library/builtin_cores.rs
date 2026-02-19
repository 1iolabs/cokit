// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::Cores;
use cid::Cid;
use std::collections::BTreeSet;

/// Get a set of built-in cores.
pub fn builtin_cores() -> BTreeSet<Cid> {
	let builtin_cores: BTreeSet<Cid> = Cores::default().built_in_native_mapping().into_keys().collect();
	builtin_cores
}
