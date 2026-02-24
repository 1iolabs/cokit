pub struct ModuleDescription {
	/// Exports (name, type).
	pub exports: Vec<(String, String)>,

	/// Imports (module, name, type).
	pub imports: Vec<(String, String, String)>,
}
impl ModuleDescription {
	#[cfg(all(feature = "fs", any(feature = "llvm", feature = "cranelift")))]
	pub async fn from_path(path: &std::path::Path) -> anyhow::Result<ModuleDescription> {
		let bytes = tokio::fs::read(path).await?;
		let (_kind, _store, module) = crate::runtimes::wasmer::WasmerRuntimeBuilder::wasm(&bytes).for_info().build()?;
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
