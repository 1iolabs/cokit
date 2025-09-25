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
		let (_store, module) = WasmerRuntimeBuilder::wasm(&bytes).for_info().build()?;
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
