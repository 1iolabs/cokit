fn main() {
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

		// llvm
		if std::env::var("LLVM_SYS_180_PREFIX").is_err() {
			match exec(std::process::Command::new("brew").arg("--prefix").arg("llvm@18")) {
				Ok(path) => {
					println!("cargo:rustc-env=LLVM_SYS_180_PREFIX={}", path);
				},
				Err(err) => {
					println!("cargo:warning=llvm failed: {}", err);
				},
			}
		}

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
