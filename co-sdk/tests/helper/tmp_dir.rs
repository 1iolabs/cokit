use std::path::{Path, PathBuf};

pub struct TmpDir {
	path: PathBuf,
}
impl TmpDir {
	pub fn new(prefix: &str) -> Self {
		Self::try_new(prefix).expect("tmp_dir")
	}

	pub fn try_new(prefix: &str) -> std::io::Result<Self> {
		let path = std::env::temp_dir().join(prefix).join(uuid::Uuid::new_v4().to_string());
		std::fs::create_dir_all(&path)?;
		Ok(Self { path })
	}

	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn clear(&self) -> std::io::Result<()> {
		std::fs::remove_dir_all(&self.path)
	}
}
impl Drop for TmpDir {
	fn drop(&mut self) {
		self.clear().ok();
	}
}
