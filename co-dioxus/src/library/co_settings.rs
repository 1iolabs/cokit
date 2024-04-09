use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct CoSettings {
	pub identifier: Option<String>,
	pub path: Option<PathBuf>,
}
impl CoSettings {
	pub fn with_path(self, path: &str) -> Self {
		Self { path: Some(path.into()), ..self }
	}

	pub fn with_identifier(self, identifier: &str) -> Self {
		Self { identifier: Some(identifier.into()), ..self }
	}
}
