// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use std::path::{Path, PathBuf};

/// Creats a temporary directory which will be deleted when the instance is dropped.
pub struct TmpDir {
	clean: bool,
	uuid: String,
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
		let uuid = uuid::Uuid::new_v4().to_string();
		let path = std::env::temp_dir().join(prefix).join(&uuid);
		std::fs::create_dir_all(&path)?;
		Ok(Self { path, clean: false, uuid })
	}

	/// The uuid of the tmp dir.
	pub fn uuid(&self) -> &str {
		&self.uuid
	}

	/// Path of the tmp dir.
	pub fn path(&self) -> &Path {
		&self.path
	}

	/// Clear the tmp dir.
	pub fn clear(&mut self) -> std::io::Result<()> {
		if !self.clean {
			std::fs::remove_dir_all(&self.path)?;
			self.clean = true;
		}
		Ok(())
	}

	/// Skip clear.
	pub fn without_clear(mut self) -> Self {
		self.clean = true;
		self
	}
}
impl Drop for TmpDir {
	fn drop(&mut self) {
		self.clear().ok();
	}
}
