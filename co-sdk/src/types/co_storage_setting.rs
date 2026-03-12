// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

#[cfg(feature = "fs")]
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum CoStorageSetting {
	/// Store data in memory
	Memory,

	/// Store data in default path
	#[cfg(feature = "fs")]
	PathDefault,

	/// Storage data in path
	#[cfg(feature = "fs")]
	Path(PathBuf),

	/// Use IndexedDb storage
	#[cfg(all(feature = "indexeddb", target_arch = "wasm32"))]
	IndexedDb,
}
#[cfg(feature = "fs")]
#[allow(clippy::derivable_impls)]
impl Default for CoStorageSetting {
	fn default() -> Self {
		CoStorageSetting::PathDefault
	}
}
#[cfg(not(feature = "fs"))]
#[allow(clippy::derivable_impls)]
impl Default for CoStorageSetting {
	fn default() -> Self {
		CoStorageSetting::Memory
	}
}
