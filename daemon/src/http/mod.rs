// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use serde::{Deserialize, Serialize};

pub mod co;
pub mod co_core;
pub mod co_cores;
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
