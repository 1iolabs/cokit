// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	co_v1::{storage_block_get, storage_block_set, CoV1Api},
	runtimes::{Runtime, RuntimeBox, RuntimeError},
	RuntimeContext,
};
use anyhow::anyhow;
use co_api::{Block, Cid, RawCid};
use co_primitives::{cid_to_raw, from_cbor, raw_to_cid, GuardOutput, KnownMultiCodec, ReducerOutput, CID_MAX_SIZE};
use co_storage::Storage;
use std::fmt::Debug;
use wasmer::{
	imports, AsStoreMut, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, Module, Store, Value, WasmPtr,
};
#[cfg(any(feature = "headless", feature = "llvm", feature = "cranelift"))]
use wasmer_types::Features;

enum RuntimeState {
	Uninitialized(bool, Vec<u8>, Option<Vec<WasmerRuntimeKind>>),
	Initialized(Box<WasmerRuntime>),
}
impl Debug for RuntimeState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Uninitialized(arg0, arg1, arg2) => f
				.debug_tuple("Uninitialized")
				.field(arg0)
				.field(&arg1.len())
				.field(arg2)
				.finish(),
			Self::Initialized(arg0) => f.debug_tuple("Initialized").field(arg0).finish(),
		}
	}
}

#[derive(Debug)]
struct Wasmer {
	state: RuntimeState,
}
impl Wasmer {
	pub fn new(native: bool, bytes: Vec<u8>) -> Self {
		Self { state: RuntimeState::Uninitialized(native, bytes, None) }
	}

	pub fn with_preferred_engines(native: bool, bytes: Vec<u8>, engines: Vec<WasmerRuntimeKind>) -> Self {
		Self { state: RuntimeState::Uninitialized(native, bytes, Some(engines)) }
	}
}
impl Runtime for Wasmer {
	/// Execute runtime with api and return new state `Cid`.
	fn execute_state(&mut self, api: CoV1Api) -> Result<RuntimeContext, RuntimeError> {
		// initialize
		let runtime: &mut WasmerRuntime = wasmer_runtime(&mut self.state)?;

		// execute
		let result = runtime.execute_state(api)?;

		// result
		Ok(result)
	}

	fn execute_guard(&mut self, api: CoV1Api) -> Result<(RuntimeContext, bool), RuntimeError> {
		// initialize
		let runtime: &mut WasmerRuntime = wasmer_runtime(&mut self.state)?;

		// execute
		let result = runtime.execute_guard(api)?;

		// result
		Ok(result)
	}
}

fn wasmer_runtime(state: &mut RuntimeState) -> Result<&mut WasmerRuntime, RuntimeError> {
	// initialize
	let runtime: &mut WasmerRuntime = match state {
		RuntimeState::Uninitialized(native, bytes, preferred) => {
			*state = RuntimeState::Initialized(Box::new(WasmerRuntime::new(*native, bytes, preferred.take())?));
			if let RuntimeState::Initialized(runtime) = state {
				runtime
			} else {
				return Err(RuntimeError::InvalidState(anyhow!("Uninitialized after initialize")));
			}
		},
		RuntimeState::Initialized(runtime) => runtime,
	};
	Ok(runtime)
}

pub fn create_runtime(native: bool, bytes: Vec<u8>) -> RuntimeBox {
	Box::new(Wasmer::new(native, bytes))
}

pub fn create_runtime_with_engines(native: bool, bytes: Vec<u8>, engines: Vec<WasmerRuntimeKind>) -> RuntimeBox {
	Box::new(Wasmer::with_preferred_engines(native, bytes, engines))
}

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
	#[error("Engine not available: {0:?}")]
	EngineNotAvailable(WasmerRuntimeKind),
	#[error("All engines failed: {0:?}")]
	AllEnginesFailed(Vec<(WasmerRuntimeKind, String)>),
}
impl From<WasmerError> for RuntimeError {
	fn from(value: WasmerError) -> Self {
		match value {
			WasmerError::Compile(e) => Self::InvalidArgument(e.into()),
			WasmerError::Instantiation(e) => Self::InvalidArgument(e.into()),
			WasmerError::Export(e) => Self::InvalidArgument(e.into()),
			WasmerError::Runtime(e) => Self::Runtime(e.into()),
			WasmerError::Deserialize(e) => Self::Deserialize(e.into()),
			e @ WasmerError::NoEngineAvailable => Self::InvalidArgument(e.into()),
			e @ WasmerError::EngineNotAvailable(_) => Self::InvalidArgument(e.into()),
			e @ WasmerError::AllEnginesFailed(_) => Self::InvalidArgument(e.into()),
		}
	}
}

impl WasmerRuntime {
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(bytes), fields(bytes.len = bytes.len()))]
	pub fn new(
		native: bool,
		bytes: &[u8],
		preferred_engines: Option<Vec<WasmerRuntimeKind>>,
	) -> Result<Self, WasmerError> {
		// module
		let mut builder = if native { WasmerRuntimeBuilder::native(bytes) } else { WasmerRuntimeBuilder::wasm(bytes) };
		if let Some(engines) = preferred_engines {
			builder = builder.with_preferred_engines(engines);
		}
		let (kind, store, module) = builder.build()?;

		// TODO: adjust check to support state or guard only binaries
		// // check
		// instance.exports.get_function("state")?;
		// instance.exports.get_function("guard")?;

		// result
		Ok(Self { kind, store, module })
	}

	/// Reset the Store to prevent unbounded growth of StoreObjects::function_environments.
	/// Each FunctionEnv::new() pushes an entry that is never removed, holding a reference to
	/// WebAssembly.Memory and preventing GC.
	fn reset(&mut self) {
		self.store = Store::new(self.store.engine().clone());
	}

	fn instance(&mut self, api: CoV1Api) -> Result<(Instance, FunctionEnv<WasmerEnv>, u32), WasmerError> {
		self.reset();
		let env = FunctionEnv::new(&mut self.store, WasmerEnv { memory: None, api });
		let import_object = Self::imports(&mut self.store, &env);
		let instance: Instance = Instance::new(&mut self.store, &self.module, &import_object)?;
		let memory = instance.exports.get_memory("memory")?.clone();

		// reserve a page for CID I/O buffers
		const WASM_PAGE_SIZE: u32 = 65536;
		let previous_pages = memory
			.grow(&mut self.store, 1)
			.map_err(|err| WasmerError::Runtime(wasmer::RuntimeError::new(err.to_string())))?;
		let cid_buffer_base = previous_pages.0 * WASM_PAGE_SIZE;

		env.as_mut(&mut self.store).memory = Some(memory);
		Ok((instance, env, cid_buffer_base))
	}

	/// Store a value as a CBOR block in the API storage. Returns the CID.
	fn store_block(api: &mut CoV1Api, data: &[u8]) -> Result<Cid, WasmerError> {
		let block = Block::new_data(KnownMultiCodec::DagCbor, data.to_vec());
		let cid = *block.cid();
		api.set(block)
			.map_err(|err| WasmerError::Runtime(wasmer::RuntimeError::new(err.to_string())))?;
		Ok(cid)
	}

	/// Write a RawCid to WASM memory at the given offset.
	fn write_raw_cid_to_memory(
		&self,
		env: &FunctionEnv<WasmerEnv>,
		offset: u32,
		raw: &RawCid,
	) -> Result<(), WasmerError> {
		let memory = env
			.as_ref(&self.store)
			.memory
			.as_ref()
			.ok_or_else(|| WasmerError::Runtime(wasmer::RuntimeError::new("no memory")))?
			.clone();
		let view = memory.view(&self.store);
		view.write(offset as u64, raw)
			.map_err(|err| WasmerError::Runtime(wasmer::RuntimeError::new(err.to_string())))?;
		Ok(())
	}

	/// Read a RawCid from WASM memory at the given offset.
	fn read_raw_cid_from_memory(&self, env: &FunctionEnv<WasmerEnv>, offset: u32) -> Result<RawCid, WasmerError> {
		let memory = env
			.as_ref(&self.store)
			.memory
			.as_ref()
			.ok_or_else(|| WasmerError::Runtime(wasmer::RuntimeError::new("no memory")))?
			.clone();
		let view = memory.view(&self.store);
		let mut buf = [0u8; CID_MAX_SIZE];
		view.read(offset as u64, &mut buf)
			.map_err(|err| WasmerError::Runtime(wasmer::RuntimeError::new(err.to_string())))?;
		Ok(buf)
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret)]
	pub fn execute_state(&mut self, api: CoV1Api) -> Result<RuntimeContext, WasmerError> {
		let (instance, env, cid_buffer_base) = self.instance(api)?;

		let input_offset = cid_buffer_base;
		let output_offset = cid_buffer_base + CID_MAX_SIZE as u32;

		// store serialized input as block
		let input_data = env.as_ref(&self.store).api.context().input.clone();
		let input_cid = Self::store_block(&mut env.as_mut(&mut self.store).api, &input_data)?;

		// write input CID to WASM memory
		let input_raw = cid_to_raw(&input_cid);
		self.write_raw_cid_to_memory(&env, input_offset, &input_raw)?;

		// call state(input_ptr, output_ptr)
		let state_fn = instance.exports.get_function("state")?;
		state_fn.call(&mut self.store, &[Value::I32(input_offset as i32), Value::I32(output_offset as i32)])?;

		// read output CID from WASM memory
		let output_raw = self.read_raw_cid_from_memory(&env, output_offset)?;
		let output_cid = raw_to_cid(&output_raw)
			.ok_or_else(|| WasmerError::Runtime(wasmer::RuntimeError::new("invalid output CID")))?;

		// load
		let output_block = env
			.as_ref(&self.store)
			.api
			.get(&output_cid)
			.map_err(|err| WasmerError::Runtime(wasmer::RuntimeError::new(err.to_string())))?;
		let reducer_output: ReducerOutput = from_cbor(output_block.data())
			.map_err(|err| WasmerError::Runtime(wasmer::RuntimeError::new(err.to_string())))?;

		// result
		let mut context = env.as_ref(&self.store).api.context().clone();
		context.apply_reducer_output(reducer_output);
		Ok(context)
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret)]
	pub fn execute_guard(&mut self, api: CoV1Api) -> Result<(RuntimeContext, bool), WasmerError> {
		let (instance, env, cid_buffer_base) = self.instance(api)?;

		let input_offset = cid_buffer_base;
		let output_offset = cid_buffer_base + CID_MAX_SIZE as u32;

		// the payload is already CBOR-encoded GuardInput — store as block directly
		let input_data = env.as_ref(&self.store).api.context().input.clone();
		let input_cid = Self::store_block(&mut env.as_mut(&mut self.store).api, &input_data)?;

		// write input CID to WASM memory
		let input_raw = cid_to_raw(&input_cid);
		self.write_raw_cid_to_memory(&env, input_offset, &input_raw)?;

		// call guard(input_ptr, output_ptr)
		let guard_fn = instance.exports.get_function("guard")?;
		guard_fn.call(&mut self.store, &[Value::I32(input_offset as i32), Value::I32(output_offset as i32)])?;

		// read output CID from WASM memory
		let output_raw = self.read_raw_cid_from_memory(&env, output_offset)?;
		let output_cid = raw_to_cid(&output_raw)
			.ok_or_else(|| WasmerError::Runtime(wasmer::RuntimeError::new("invalid output CID")))?;

		// load GuardOutput
		let output_block = env
			.as_ref(&self.store)
			.api
			.get(&output_cid)
			.map_err(|err| WasmerError::Runtime(wasmer::RuntimeError::new(err.to_string())))?;
		let guard_output: GuardOutput = from_cbor(output_block.data())
			.map_err(|err| WasmerError::Runtime(wasmer::RuntimeError::new(err.to_string())))?;

		// result
		let result = guard_output.result;
		let mut context = env.as_ref(&self.store).api.context().clone();
		context.apply_guard_output(guard_output);
		Ok((context, result))
	}

	fn imports(store: &mut impl AsStoreMut, env: &FunctionEnv<WasmerEnv>) -> wasmer::Imports {
		imports! {
			"co_v1" => {
				"storage_block_get" => Function::new_typed_with_env(store, env, wasmer_storage_block_get),
				"storage_block_set" => Function::new_typed_with_env(store, env, wasmer_storage_block_set),
			}
		}
	}
}

/// Initiate a WASM (or AOT native) module.
/// Attempts to pick the most optimal runtime which is available.
/// If compilation fails for one engine, falls back to the next available engine.
///
/// See:
/// - https://github.com/wasmerio/wasmer/blob/dcaff6c83316e9e67b62ade47e70a9b121c08b15/lib/cli/src/backend.rs#L670
pub struct WasmerRuntimeBuilder<'a> {
	#[cfg(feature = "headless")]
	native: bool,
	bytes: &'a [u8],
	preferred_engines: Option<Vec<WasmerRuntimeKind>>,
}
impl<'a> WasmerRuntimeBuilder<'a> {
	pub fn wasm(bytes: &'a [u8]) -> Self {
		Self {
			#[cfg(feature = "headless")]
			native: false,
			bytes,
			preferred_engines: None,
		}
	}

	pub fn native(bytes: &'a [u8]) -> Self {
		Self {
			#[cfg(feature = "headless")]
			native: true,
			bytes,
			preferred_engines: None,
		}
	}

	pub fn with_preferred_engines(mut self, engines: Vec<WasmerRuntimeKind>) -> Self {
		self.preferred_engines = Some(engines);
		self
	}

	#[cfg(any(feature = "headless", feature = "llvm", feature = "cranelift"))]
	fn features() -> Features {
		let mut features = Features::none();
		features.reference_types = true;
		features.bulk_memory = true;
		features.multi_value = true;
		features.extended_const = true;
		features
	}

	/// Try to compile the WASM module using a specific engine.
	#[allow(unreachable_code, unused_variables)]
	fn try_engine(&self, kind: WasmerRuntimeKind) -> Result<(WasmerRuntimeKind, Store, Module), WasmerError> {
		match kind {
			WasmerRuntimeKind::Llvm => {
				#[cfg(feature = "llvm")]
				if !is_sandboxed() {
					let mut config = wasmer_compiler_llvm::LLVM::default();
					wasmer_compiler::CompilerConfig::canonicalize_nans(&mut config, true);
					let engine: wasmer::Engine = wasmer::sys::EngineBuilder::new(config)
						.set_features(Some(Self::features()))
						.engine()
						.into();
					let store = Store::new(engine);
					let module = Module::new(&store, self.bytes)?;
					return Ok((WasmerRuntimeKind::Llvm, store, module));
				}
				Err(WasmerError::EngineNotAvailable(kind))
			},
			WasmerRuntimeKind::Cranelift => {
				#[cfg(feature = "cranelift")]
				if !is_sandboxed() {
					let mut config = wasmer_compiler_cranelift::Cranelift::new();
					wasmer_compiler::CompilerConfig::canonicalize_nans(&mut config, true);
					let engine: wasmer::Engine = wasmer::sys::EngineBuilder::new(config)
						.set_features(Some(Self::features()))
						.engine()
						.into();
					let store = Store::new(engine);
					let module = Module::new(&store, self.bytes)?;
					return Ok((WasmerRuntimeKind::Cranelift, store, module));
				}
				Err(WasmerError::EngineNotAvailable(kind))
			},
			WasmerRuntimeKind::Wamr => {
				#[cfg(feature = "wamr")]
				{
					let engine: wasmer::Engine = wasmer::wamr::Wamr::new().into();
					let store = Store::new(engine);
					let module = Module::new(&store, self.bytes)?;
					return Ok((WasmerRuntimeKind::Wamr, store, module));
				}
				#[allow(unreachable_code)]
				Err(WasmerError::EngineNotAvailable(kind))
			},
			WasmerRuntimeKind::Wasmi => {
				#[cfg(feature = "wasmi")]
				{
					let engine: wasmer::Engine = wasmer::wasmi::Wasmi::new().into();
					let store = Store::new(engine);
					let module = Module::new(&store, self.bytes)?;
					return Ok((WasmerRuntimeKind::Wasmi, store, module));
				}
				#[allow(unreachable_code)]
				Err(WasmerError::EngineNotAvailable(kind))
			},
			WasmerRuntimeKind::Jsc => {
				#[cfg(all(feature = "jsc", target_vendor = "apple"))]
				{
					let engine = wasmer::jsc::JSC::default();
					let store = Store::new(engine);
					let module = Module::new(&store, self.bytes)?;
					return Ok((WasmerRuntimeKind::Jsc, store, module));
				}
				#[allow(unreachable_code)]
				Err(WasmerError::EngineNotAvailable(kind))
			},
			// Platform-specific kinds are not fallback candidates
			_ => Err(WasmerError::EngineNotAvailable(kind)),
		}
	}

	#[allow(unreachable_code)]
	pub fn build(self) -> Result<(WasmerRuntimeKind, Store, Module), WasmerError> {
		// Platform-specific paths: no fallback, return immediately.

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
				.set_features(Some(Self::features()))
				.engine()
				.into();
			let store = Store::new(engine);
			let module = unsafe { Module::deserialize(&store, self.bytes)? };
			return Ok((WasmerRuntimeKind::Headless, store, module));
		}

		// Compiler backends: try each with fallback on compilation failure.
		let default_order = [
			WasmerRuntimeKind::Llvm,
			WasmerRuntimeKind::Cranelift,
			WasmerRuntimeKind::Jsc,
			WasmerRuntimeKind::Wamr,
			WasmerRuntimeKind::Wasmi,
		];
		let engine_order: &[WasmerRuntimeKind] = match &self.preferred_engines {
			Some(engines) => engines,
			None => &default_order,
		};

		let mut errors: Vec<(WasmerRuntimeKind, String)> = Vec::new();

		for &kind in engine_order {
			match self.try_engine(kind) {
				Ok(result) => return Ok(result),
				Err(WasmerError::EngineNotAvailable(_)) => {
					// engine not compiled in or not usable — skip silently
				},
				Err(err) => {
					tracing::warn!(?kind, ?err, "engine compilation failed, trying next");
					errors.push((kind, err.to_string()));
				},
			}
		}

		if errors.is_empty() {
			Err(WasmerError::NoEngineAvailable)
		} else {
			Err(WasmerError::AllEnginesFailed(errors))
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmerRuntimeKind {
	Headless,
	Llvm,
	Cranelift,
	Wamr,
	Wasmi,
	Js,
	Jsc,
}

#[cfg(any(feature = "llvm", feature = "cranelift"))]
fn is_sandboxed() -> bool {
	std::env::var("APP_SANDBOX_CONTAINER_ID").is_ok()
}

fn into_runtime_error<E: ToString>(err: E) -> wasmer::RuntimeError {
	wasmer::RuntimeError::new(err.to_string())
}

fn wasmer_storage_block_get(
	mut env: FunctionEnvMut<WasmerEnv>,
	cid: WasmPtr<u8>,
	cid_size: u32,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
) -> Result<u32, wasmer::RuntimeError> {
	let (data, store) = env.data_and_store_mut();
	let memory = data
		.memory
		.as_ref()
		.ok_or_else(|| wasmer::RuntimeError::new("no memory"))?
		.view(&store);
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
) -> Result<u32, wasmer::RuntimeError> {
	let (data, store) = env.data_and_store_mut();
	let memory = data
		.memory
		.as_ref()
		.ok_or_else(|| wasmer::RuntimeError::new("no memory"))?
		.view(&store);
	let cid_access = cid.slice(&memory, cid_size)?.access()?;
	let buffer_access = buffer.slice(&memory, buffer_size)?.access()?;
	storage_block_set(&mut data.api, cid_access.as_ref(), buffer_access.as_ref()).map_err(into_runtime_error)
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
}
