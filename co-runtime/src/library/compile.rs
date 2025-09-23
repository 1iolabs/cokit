use core::str::FromStr;
use wasmer::{Module, Store};
use wasmer_compiler::{
	types::target::{CpuFeature, Target, Triple},
	EngineBuilder,
};
use wasmer_compiler_llvm::LLVM;

pub async fn compile_native(wasm_bytes: impl AsRef<[u8]>, arch_triple: &str) -> Result<Vec<u8>, anyhow::Error> {
	let mut compiler = EngineBuilder::new(LLVM::default());
	match arch_triple {
		"aarch64-apple-darwin" if cfg!(target_arch = "aarch64") && cfg!(target_vendor = "apple") => {},
		"x86_64-apple-darwin" if cfg!(target_arch = "x86_64") && cfg!(target_vendor = "apple") => {},
		_ => {
			let target = Triple::from_str(arch_triple)
				.map_err(|err| anyhow::anyhow!("Parse triple failed: {}: {}", arch_triple, err.to_string()))?;
			let target = Target::new(target, CpuFeature::for_host().into());
			compiler = compiler.set_target(Some(target));
			// compiler.target_machine(&target)
			// compiler = LLVM::for_traget(target);
		},
	}
	let engine = compiler.engine();
	let store = Store::new(engine);
	let module = Module::new(&store, wasm_bytes)?;
	Ok(module.serialize()?.to_vec())
}
