use crate::co_v1::{event_cid_read, state_cid_read, state_cid_write, storage_block_get, storage_block_set, CoV1Api};
use wasmer::{imports, AsStoreMut, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, Module, Store, WasmPtr};

pub struct WasmerRuntime {
	store: Store,
	instance: Instance,
	env: FunctionEnv<WasmerEnv>,
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
}

impl WasmerRuntime {
	pub fn new(api: CoV1Api, bytes: Vec<u8>) -> Result<Self, WasmerError> {
		let mut store = Store::default();

		// env
		let env = FunctionEnv::new(&mut store, WasmerEnv { memory: None, api });

		// module
		let module = Module::new(&store, &bytes)?;

		// instance
		let import_object = Self::imports(&mut store, &env);
		let instance: Instance = Instance::new(&mut store, &module, &import_object)?;
		let memory = instance.exports.get_memory("memory").unwrap().clone();
		env.as_mut(&mut store).memory = Some(memory);

		// check
		instance.exports.get_function("state")?;

		// result
		Ok(Self { store, instance, env })
	}

	pub fn execute(&mut self) -> Result<(), WasmerError> {
		let state = self.instance.exports.get_function("state").unwrap();
		state.call(&mut self.store, &[])?;
		Ok(())
	}

	pub fn api(&self) -> &CoV1Api {
		&self.env.as_ref(&self.store).api
	}

	fn imports(store: &mut impl AsStoreMut, env: &FunctionEnv<WasmerEnv>) -> wasmer::Imports {
		imports! {
			"co_v1" => {
				"storage_block_get" => Function::new_typed_with_env(store, env, wasmer_storage_block_get),
				"storage_block_set" => Function::new_typed_with_env(store, env, wasmer_storage_block_set),
				"state_cid_read" => Function::new_typed_with_env(store, env, wasmer_state_cid_read),
				"state_cid_write" => Function::new_typed_with_env(store, env, wasmer_state_cid_write),
				"event_cid_read" => Function::new_typed_with_env(store, env, wasmer_event_cid_read),
			}
		}
	}
}

fn wasmer_storage_block_get(
	mut env: FunctionEnvMut<WasmerEnv>,
	cid: WasmPtr<u8>,
	cid_size: u32,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
) -> u32 {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().unwrap().view(&store);
	let cid_access = cid
		.slice(&memory, cid_size)
		.expect("pointer in bounds")
		.access()
		.expect("pointer in bounds");
	let mut buffer_access = buffer
		.slice(&memory, buffer_size)
		.expect("pointer in bounds")
		.access()
		.expect("pointer in bounds");
	storage_block_get(&mut data.api, cid_access.as_ref(), buffer_access.as_mut())
		.expect("to not have internal errors")
		.try_into()
		.expect("API")
	// loop {
	// 	let result = storage_block_get(
	// 		env.data_mut(),
	// 		cid_buffer.access().expect("pointer in bounds").as_ref(),
	// 		buffer.access().expect("pointer in bounds").as_mut(),
	// 	);
	// 	return match result {
	// 		Ok(i) => i,
	// 		Err(e) if e.is_retriable() => {
	// 			// TODO: Add some backoff. Maybe via CoV1Api?
	// 			continue
	// 		},
	// 		Err(e) => Err(e).unwrap(),
	// 	}
	// }
}

fn wasmer_storage_block_set(
	mut env: FunctionEnvMut<WasmerEnv>,
	cid: WasmPtr<u8>,
	cid_size: u32,
	buffer: WasmPtr<u8>,
	buffer_size: u32,
) -> u32 {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().unwrap().view(&store);
	let cid_access = cid
		.slice(&memory, cid_size)
		.expect("pointer in bounds")
		.access()
		.expect("pointer in bounds");
	let buffer_access = buffer
		.slice(&memory, buffer_size)
		.expect("pointer in bounds")
		.access()
		.expect("pointer in bounds");
	storage_block_set(&mut data.api, cid_access.as_ref(), buffer_access.as_ref()).expect("API")
}
fn wasmer_state_cid_read(mut env: FunctionEnvMut<WasmerEnv>, buffer: WasmPtr<u8>, buffer_size: u32) -> u32 {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().unwrap().view(&store);
	let mut buffer_access = buffer
		.slice(&memory, buffer_size)
		.expect("pointer in bounds")
		.access()
		.expect("pointer in bounds");
	state_cid_read(&data.api, buffer_access.as_mut())
}
fn wasmer_state_cid_write(mut env: FunctionEnvMut<WasmerEnv>, buffer: WasmPtr<u8>, buffer_size: u32) -> u32 {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().unwrap().view(&store);
	let buffer_access = buffer
		.slice(&memory, buffer_size)
		.expect("pointer in bounds")
		.access()
		.expect("pointer in bounds");
	state_cid_write(&mut data.api, buffer_access.as_ref()).expect("API")
}
fn wasmer_event_cid_read(mut env: FunctionEnvMut<WasmerEnv>, buffer: WasmPtr<u8>, buffer_size: u32) -> u32 {
	let (data, store) = env.data_and_store_mut();
	let memory = data.memory.as_ref().unwrap().view(&store);
	let mut buffer_access = buffer
		.slice(&memory, buffer_size)
		.expect("pointer in bounds")
		.access()
		.expect("pointer in bounds");
	event_cid_read(&data.api, buffer_access.as_mut())
}

#[cfg(test)]
mod tests {
	use wasmer::{imports, Function, FunctionEnv, FunctionEnvMut, Instance, Module, Store, Value};

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
		let module = Module::new(&store, &module_wat)?;

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
