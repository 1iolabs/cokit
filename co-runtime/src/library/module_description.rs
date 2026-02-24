// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::runtimes::wasmer::WasmerRuntimeBuilder;
use std::path::Path;

pub struct ModuleDescription {
	/// Exports (name, type).
	pub exports: Vec<(String, String)>,

	/// Imports (module, name, type).
	pub imports: Vec<(String, String, String)>,
}
impl ModuleDescription {
	pub async fn from_path(path: &Path) -> anyhow::Result<ModuleDescription> {
		let bytes = tokio::fs::read(path).await?;
		let (_kind, _store, module) = WasmerRuntimeBuilder::wasm(&bytes).for_info().build()?;
		Ok(ModuleDescription {
			exports: module
				.exports()
				.map(|export| (export.name().to_owned(), format!("{:?}", export.ty())))
				.collect(),
			imports: module
				.imports()
				.map(|import| (import.module().to_owned(), import.name().to_string(), format!("{:?}", import.ty())))
				.collect(),
		})
	}
}
