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
