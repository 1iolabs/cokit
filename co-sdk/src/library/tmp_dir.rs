use std::path::{Path, PathBuf};

/// Creats a temporary directory which will be deleted when the instance is dropped.
pub struct TmpDir {
	path: PathBuf,
}
impl TmpDir {
	/// Create a tmp dir using a custom prefix.
	///
	/// Panics:
	/// - The tmp dir could not be created.
	pub fn new(prefix: &str) -> Self {
		Self::try_new(prefix).expect("tmp_dir")
	}

	/// Create a tmp dir using a custom prefix.
	pub fn try_new(prefix: &str) -> std::io::Result<Self> {
		let path = std::env::temp_dir().join(prefix).join(uuid::Uuid::new_v4().to_string());
		std::fs::create_dir_all(&path)?;
		Ok(Self { path })
	}

	/// Path of the tmp dir.
	pub fn path(&self) -> &Path {
		&self.path
	}

	/// Clear the tmp dir.
	pub fn clear(&self) -> std::io::Result<()> {
		std::fs::remove_dir_all(&self.path)
	}
}
impl Drop for TmpDir {
	fn drop(&mut self) {
		self.clear().ok();
	}
}
