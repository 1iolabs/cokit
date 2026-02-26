// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	co_v1::{
		diagnostic_cid_write, event_cid_read, payload_read, state_cid_read, state_cid_write, storage_block_get,
		storage_block_set, CoV1Api,
	},
	RuntimeContext,
};
use std::fmt::Debug;
use wasmer::{
	imports, AsStoreMut, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, Module, RuntimeError, Store, WasmPtr,
};
use wasmer_types::Features;

pub struct WasmerRuntime {
	store: Store,
	module: Module,
	kind: WasmerRuntimeKind,
}
impl Debug for WasmerRuntime {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("WasmerRuntime")
			// .field("store", &self.store)
			// .field("instance", &self.instance)
			// .field("api", &self.api)
			.field("kind", &self.kind)
			.finish()
	}
}

pub struct WasmerEnv {
	memory: Option<Memory>,
	api: CoV1Api,
}

#[derive(Debug, thiserror::Error)]
pub enum WasmerError {
	#[error("Compile")]
	Compile(#[from] wasmer::CompileError),
	#[error("Instantiation")]
	Instantiation(#[from] wasmer::InstantiationError),
	#[error("Export")]
	Export(#[from] wasmer::ExportError),
	#[error("Runtime")]
	Runtime(#[from] wasmer::RuntimeError),
	#[error("Deserialize")]
	Deserialize(#[from] wasmer::DeserializeError),
	#[error("No engine available")]
	NoEngineAvailable,
}

impl WasmerRuntime {
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(bytes), fields(bytes.len = bytes.len()))]
	pub fn new(native: bool, bytes: &[u8]) -> Result<Self, WasmerError> {
		// module
		let (kind, store, module) =
			if native { WasmerRuntimeBuilder::native(bytes) } else { WasmerRuntimeBuilder::wasm(bytes) }.build()?;

		// TODO: adjust check to support state or guard only binaries
		// // check
		// instance.exports.get_function("state")?;
		// instance.exports.get_function("guard")?;

		// result
		Ok(Self { kind, store, module })
	}

	fn instance(&mut self, api: CoV1Api) -> Result<(Instance, FunctionEnv<WasmerEnv>), WasmerError> {
		// Reset the Store to prevent unbounded growth of StoreObjects::function_environments.
		// Each FunctionEnv::new() pushes an entry that is never removed, holding a reference to
		// WebAssembly.Memory and preventing GC. On the JS backend, Store is lightweight (empty Engine)
		// and Module is an independent WebAssembly.Module, so resetting is safe and nearly free.
		#[cfg(feature = "js")]
		{
			self.store = Store::default();
		}
		let env = FunctionEnv::new(&mut self.store, WasmerEnv { memory: None, api });
		let import_object = Self::imports(&mut self.store, &env);
		let instance: Instance = Instance::new(&mut self.store, &self.module, &import_object)?;
		let memory = instance.exports.get_memory("memory")?.clone();
		env.as_mut(&mut self.store).memory = Some(memory);
		Ok((instance, env))
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret)]
	pub fn execute_state(&mut self, api: CoV1Api) -> Result<RuntimeContext, WasmerError> {
		let (instance, env) = self.instance(api)?;
		let state = instance.exports.get_function("state")?;
		state.call(&mut self.store, &[])?;
		Ok(env.as_ref(&self.store).api.context().clone())
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret)]
	pub fn execute_guard(&mut self, api: CoV1Api) -> Result<bool, WasmerError> {
		let (instance, _) = self.instance(api)?;
		let state = instance.exports.get_function("guard")?;
		let results = state.call(&mut self.store, &[])?;
		if results.len() != 1 {
			return Err(wasmer::ExportError::IncompatibleType.into());
		}
		let result = results
			.first()
			.ok_or(wasmer::ExportError::IncompatibleType)?
			.i32()
			.ok_or(wasmer::ExportError::IncompatibleType)?;
		Ok(result == 1)
	}

	fn imports(store: &mut impl AsStoreMut, env: &FunctionEnv<WasmerEnv>) -> wasmer::Imports {
		imports! {
			"co_v1" => {
				"storage_block_get" => Function::new_typed_with_env(store, env, wasmer_storage_block_get),
				"storage_block_set" => Function::new_typed_with_env(store, env, wasmer_storage_block_set),
				"state_cid_read" => Function::new_typed_with_env(store, env, wasmer_state_cid_read),
				"state_cid_write" => Function::new_typed_with_env(store, env, wasmer_state_cid_write),
				"event_cid_read" => Function::new_typed_with_env(store, env, wasmer_event_cid_read),
				"payload_read" => Function::new_typed_with_env(store, env, wasmer_payload_read),
				"diagnostic_cid_write" => Function::new_typed_with_env(store, env, wasmer_diagnostic_cid_write),
			}
		}
	}
}

/// Initiate a WASM (or AOT native) module.
/// Attempts to pick the most optimal runtime which is available.
///
/// See:
/// - https://github.com/wasmerio/wasmer/blob/dcaff6c83316e9e67b62ade47e70a9b121c08b15/lib/cli/src/backend.rs#L670
pub struct WasmerRuntimeBuilder<'a> {
	#[cfg(feature = "headless")]
	native: bool,
	bytes: &'a [u8],
	#[cfg(any(feature = "llvm", feature = "cranelift"))]
	llvm: bool,
}
impl<'a> WasmerRuntimeBuilder<'a> {
	pub fn wasm(bytes: &'a [u8]) -> Self {
		Self {
			#[cfg(feature = "headless")]
			native: false,
			bytes,
			#[cfg(any(feature = "llvm", feature = "cranelift"))]
			llvm: true,
		}
	}

	pub fn native(bytes: &'a [u8]) -> Self {
		Self {
			#[cfg(feature = "headless")]
			native: true,
			bytes,
			#[cfg(any(feature = "llvm", feature = "cranelift"))]
			llvm: true,
		}
	}

	#[cfg(any(feature = "llvm", feature = "cranelift"))]
	pub fn for_info(mut self) -> Self {
		self.llvm = false;
		self
	}

	#[allow(unreachable_code)]
	pub fn build(self) -> Result<(WasmerRuntimeKind, Store, Module), WasmerError> {
		let mut features = Features::none();
		features.reference_types = true;
		features.bulk_memory = true;
		features.multi_value = true;
		features.extended_const = true;

		// js
		#[cfg(feature = "js")]
		{
			let store = Store::default();
			let module = unsafe { Module::deserialize(&store, self.bytes)? };
			return Ok((WasmerRuntimeKind::Js, store, module));
		}

		// bytes are native code
		#[cfg(feature = "headless")]
		if self.native {
			let engine: wasmer::Engine = wasmer::sys::EngineBuilder::headless()
				.set_features(Some(features))
				.engine()
				.into();
			let store = Store::new(engine);
			let module = unsafe { Module::deserialize(&store, self.bytes)? };
			return Ok((WasmerRuntimeKind::Headless, store, module));
		}

		// llvm feature
		#[cfg(feature = "llvm")]
		if self.llvm && !is_sandboxed() {
			let mut config = wasmer_compiler_llvm::LLVM::default();
			wasmer_compiler::CompilerConfig::canonicalize_nans(&mut config, true);
			// config.opt_level(wasmer_compiler_llvm::LLVMOptLevel::None);
			// config.enable_verifier();
			let engine: wasmer::Engine = wasmer::sys::EngineBuilder::new(config)
				.set_features(Some(features))
				.engine()
				.into();
			let store = Store::new(engine);
			let module = Module::new(&store, self.bytes)?;
			return Ok((WasmerRuntimeKind::Llvm, store, module));
		}

		// cranelift feature
		#[cfg(feature = "cranelift")]
		if self.llvm && !is_sandboxed() {
			let mut config = wasmer_compiler_cranelift::Cranelift::new();
			wasmer_compiler::CompilerConfig::canonicalize_nans(&mut config, true);
			let engine: wasmer::Engine = wasmer::sys::EngineBuilder::new(config)
				.set_features(Some(features))
				.engine()
				.into();
			let store = Store::new(engine);
			let module = Module::new(&store, self.bytes)?;
			return Ok((WasmerRuntimeKind::Cranelift, store, module));
		}

		// wamr (WebAssembly Micro Runtime) feature
		// See: https://wasmer.io/posts/introducing-wasmer-v5
		#[cfg(feature = "wamr")]
		{
			let engine: wasmer::Engine = wasmer::wamr::Wamr::new().into();
			let store = Store::new(engine);
			let module = Module::new(&store, self.bytes)?;
			return Ok((WasmerRuntimeKind::Wamr, store, module));
		}

		// wasmi feature
		#[cfg(feature = "wasmi")]
		{
			let engine: wasmer::Engine = wasmer::wasmi::Wasmi::new().into();
			let store = Store::new(engine);
			let module = Module::new(&store, self.bytes)?;
			return Ok((WasmerRuntimeKind::Wasmi, store, module));
		}

		// none
		Err(WasmerError::NoEngineAvailable)
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum WasmerRuntimeKind {
	Headless,
	Llvm,
	Cranelift,
	Wamr,
	Wasmi,
	Js,
}

#[cfg(any(feature = "llvm", feature = "cranelift"))]
fn is_sandboxed() -> bool {
	std::env::var("APP_SANDBOX_CONTAINER_ID").is_ok()
}

fn into_runtime_error<E: ToString>(err: E) -> RuntimeError {
	RuntimeError::new(err.to_string())
}

fn wasmer_storage_block_get(
	mut env: FunctionEnvMut<WasmerEnv>,
	cid: WasmPtr<u8>,
	cid_size: u32,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
) -> Result<u32, RuntimeError> {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().ok_or_else(|| RuntimeError::new("no memory"))?.view(&store);
	let cid_access = cid.slice(&memory, cid_size)?.access()?;
	let mut buffer_access = buffer.slice(&memory, buffer_size)?.access()?;
	storage_block_get(&mut data.api, cid_access.as_ref(), buffer_access.as_mut()).map_err(into_runtime_error)
}

fn wasmer_storage_block_set(
	mut env: FunctionEnvMut<WasmerEnv>,
	cid: WasmPtr<u8>,
	cid_size: u32,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
) -> Result<u32, RuntimeError> {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().ok_or_else(|| RuntimeError::new("no memory"))?.view(&store);
	let cid_access = cid.slice(&memory, cid_size)?.access()?;
	let buffer_access = buffer.slice(&memory, buffer_size)?.access()?;
	storage_block_set(&mut data.api, cid_access.as_ref(), buffer_access.as_ref()).map_err(into_runtime_error)
}

fn wasmer_payload_read(
	mut env: FunctionEnvMut<WasmerEnv>,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
	offset: u32,
) -> Result<u32, RuntimeError> {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().ok_or_else(|| RuntimeError::new("no memory"))?.view(&store);
	let mut buffer_access = buffer.slice(&memory, buffer_size)?.access()?;
	payload_read(&data.api, buffer_access.as_mut(), offset).map_err(into_runtime_error)
}

fn wasmer_state_cid_read(
	mut env: FunctionEnvMut<WasmerEnv>,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
) -> Result<u32, RuntimeError> {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().ok_or_else(|| RuntimeError::new("no memory"))?.view(&store);
	let mut buffer_access = buffer.slice(&memory, buffer_size)?.access()?;
	state_cid_read(&data.api, buffer_access.as_mut()).map_err(into_runtime_error)
}

fn wasmer_state_cid_write(
	mut env: FunctionEnvMut<WasmerEnv>,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
) -> Result<u32, RuntimeError> {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().ok_or_else(|| RuntimeError::new("no memory"))?.view(&store);
	let buffer_access = buffer.slice(&memory, buffer_size)?.access()?;
	state_cid_write(&mut data.api, buffer_access.as_ref()).map_err(into_runtime_error)
}

fn wasmer_event_cid_read(
	mut env: FunctionEnvMut<WasmerEnv>,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
) -> Result<u32, RuntimeError> {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().ok_or_else(|| RuntimeError::new("no memory"))?.view(&store);
	let mut buffer_access = buffer.slice(&memory, buffer_size)?.access()?;
	event_cid_read(&data.api, buffer_access.as_mut()).map_err(into_runtime_error)
}

fn wasmer_diagnostic_cid_write(
	mut env: FunctionEnvMut<WasmerEnv>,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
) -> Result<u32, RuntimeError> {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().ok_or_else(|| RuntimeError::new("no memory"))?.view(&store);
	let buffer_access = buffer.slice(&memory, buffer_size)?.access()?;
	diagnostic_cid_write(&mut data.api, buffer_access.as_ref()).map_err(into_runtime_error)
}

#[cfg(test)]
mod tests {
	use wasmer::{imports, Function, FunctionEnv, FunctionEnvMut, Instance, Module, RuntimeError, Store, Value};

	#[test]
	fn wasmer_with_imports_example() -> anyhow::Result<()> {
		let module_wat = r#"
        (module
            (func $test (import "env" "test") (param i32) (result i32))

            (type $t0 (func (param i32) (result i32)))
            (func $add (export "add") (type $t0) (param $p0 i32) (result i32)
                (call $test (local.get $p0))
                i32.const 1
                i32.add
            )
        )
        "#;

		struct Env {
			pub magic: i32,
		}
		fn test(mut env: FunctionEnvMut<Env>, a: i32) -> i32 {
			let result = a + env.data().magic;
			env.data_mut().magic = env.data().magic * 2;
			result
		}

		let mut store = Store::default();
		let module = Module::new(&store, module_wat)?;

		let env = FunctionEnv::new(&mut store, Env { magic: 42 });
		let func = Function::new_typed_with_env(&mut store, &env, test);

		let import_object = imports! {
			"env" => {
				"test" => func,
			}
		};
		let instance = Instance::new(&mut store, &module, &import_object)?;

		let add = instance.exports.get_function("add")?;
		let result = add.call(&mut store, &[Value::I32(1)])?;
		assert_eq!(result[0], Value::I32(44));
		let result = add.call(&mut store, &[Value::I32(1)])?;
		assert_eq!(result[0], Value::I32(86));
		Ok(())
	}

	/// Test error handling behaviour of wasmer.
	///
	/// Note: panic! from a host function will not be handled inside wasm but will be panic the rust code.
	#[test]
	fn wasmer_panic() {
		let mut store = Store::default();
		let module = Module::new(
			&store,
			r#"
		        (module
		            (import "env" "host_func" (func $host_func))
		            (func (export "panic_host") call $host_func)
					(func (export "panic_wasm")
	    				unreachable
	        		)
		        )
		    "#,
		)
		.unwrap();

		let host_func = Function::new_typed(&mut store, || -> Result<(), RuntimeError> {
			Err(RuntimeError::new("oops from host import"))
		});

		let import_object = imports! {
			"env" => {
				"host_func" => host_func,
			}
		};
		let instance = Instance::new(&mut store, &module, &import_object).unwrap();

		// panic from WASM
		let result = instance.exports.get_function("panic_wasm").unwrap().call(&mut store, &[]);
		println!("panic_wasm result: {:?}", result);
		assert!(result.is_err());

		// panic from Host
		let result = instance.exports.get_function("panic_host").unwrap().call(&mut store, &[]);
		println!("panic_host: {:?}", result);
		assert!(result.is_err());
	}

	// #[test]
	// fn wasmer_memeory_example() {
	// 	struct Env {
	// 		memory: Option<Memory>,
	// 	}
	// 	pub fn host_import(mut env: FunctionEnvMut<Env>, ptr: WasmPtr<u32>) {
	// 		let memory = env.data().memory.unwrap().view(&env);
	// 		let derefed_ptr = ptr.deref(&memory);
	// 		let inner_val: u32 = derefed_ptr.read().expect("pointer in bounds");
	// 		println!("Got {} from Wasm memory address 0x{:X}", inner_val, ptr.offset());
	// 		// update the value being pointed to
	// 		derefed_ptr.write(inner_val + 1).expect("pointer in bounds");
	// 	}

	// 	let mut store = Store::default();
	// 	let env = FunctionEnv::new(&mut store, Env { memory: None });
	// 	let func = Function::new_typed_with_env(&mut store, &env, host_import);

	// 	// let instance = Instance::new(&mut store, &module, &import_fns).unwrap();
	// 	// let memory = instance.exports.get_memory("memory").unwrap().clone();
	// 	// env.as_mut(&mut store).memory = Some(memory);
	// }
}
