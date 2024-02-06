use serde::{Deserialize, Serialize};

pub mod co;
pub mod cos;

/// Get version info route.
/// Route: /
pub async fn get() -> axum::response::Json<VersionInfo> {
	axum::response::Json(VersionInfo { name: "co", version: "0.0.1", commit: "" })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
	name: &'static str,
	version: &'static str,
	commit: &'static str,
}
