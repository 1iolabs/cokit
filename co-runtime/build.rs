// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

fn main() {
	// emit `wasmer_backend` cfg when any wasmer backend is active
	println!("cargo:rustc-check-cfg=cfg(wasmer_backend)");
	let has_feature = |f: &str| std::env::var(format!("CARGO_FEATURE_{}", f.to_uppercase())).is_ok();
	let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
	let target_vendor = std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
	if has_feature("headless")
		|| has_feature("llvm")
		|| has_feature("cranelift")
		|| has_feature("wasmi")
		|| has_feature("wamr")
		|| (has_feature("js") && target_arch == "wasm32")
		|| (has_feature("jsc") && target_vendor == "apple")
	{
		println!("cargo:rustc-cfg=wasmer_backend");
	}

	// try to use homebrew for dependencies
	#[cfg(all(target_os = "macos", feature = "llvm"))]
	{
		fn exec(command: &mut std::process::Command) -> Result<String, String> {
			let output = command.output().map_err(|e| e.to_string())?;
			if !output.status.success() {
				return Err(format!("exec failed: {:?}: {:?}", output.status, command));
			}
			let stdout = std::str::from_utf8(&output.stdout).map_err(|e| e.to_string())?;
			Ok(stdout.trim().to_owned())
		}

		// rerun
		println!("cargo:rerun-if-changed=build.rs");

		// zstd
		match exec(std::process::Command::new("brew").arg("--prefix").arg("zstd")) {
			Ok(path) => {
				println!("cargo:rustc-link-search=native={}/lib", path);
			},
			Err(err) => {
				println!("cargo:warning=zstd failed: {}", err);
			},
		}
	}
}
