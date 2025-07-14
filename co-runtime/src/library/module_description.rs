use std::path::Path;
use wasmer::{Module, Store};

pub struct ModuleDescription {
	/// Exports (name, type).
	pub exports: Vec<(String, String)>,

	/// Imports (module, name, type).
	pub imports: Vec<(String, String, String)>,
}
impl ModuleDescription {
	pub async fn from_path(path: &Path) -> anyhow::Result<ModuleDescription> {
		let bytes = tokio::fs::read(path).await?;
		let store = Store::default();
		let module = Module::new(&store, &bytes)?;
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
