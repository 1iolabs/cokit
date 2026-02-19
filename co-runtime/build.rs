// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
