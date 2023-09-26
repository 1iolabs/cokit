use directories::ProjectDirs;
use std::{io::ErrorKind, path::PathBuf};
use tokio::fs::create_dir_all;

#[derive(Debug, thiserror::Error)]
pub enum PathError {
	#[error("No home path could be retrived from OS")]
	NoHome,
}

// /// The data base path.
// pub fn base_path(base_path: Option<PathBuf>) -> Result<PathBuf, PathError> {
// 	if let Some(base_path) = base_path {
// 		return Ok(base_path)
// 	}
// 	let path = match ProjectDirs::from("CO") {
// 		Some(i) => i,
// 		None => return Err(VarError::NotPresent),
// 	};
// 	return Ok(path.data_local_dir().to_owned())
// 	// match std::env::consts::OS {
// 	// 	"macos" | "ios" => Path::new("/Users")
// 	// 		.join(std::env::var("USER")?)
// 	// 		.join("Library/Application Support/CO"),
// 	// 	"windows" => Path::new(),
// 	// }
// }

fn project_dirs() -> Result<ProjectDirs, PathError> {
	match ProjectDirs::from("co.app", "1io", "CO") {
		Some(i) => Ok(i),
		None => return Err(PathError::NoHome),
	}
}

/// The content-address storage path.
///
/// Default: <base_path>/storage
pub fn storage_path(base_path: &Option<PathBuf>, storage_path: &Option<PathBuf>) -> Result<PathBuf, PathError> {
	// custom?
	if let Some(storage_path) = storage_path {
		return Ok(storage_path.clone())
	}

	// base
	if let Some(base_path) = base_path {
		return Ok(base_path.join("storage"))
	}

	// local app data
	Ok(project_dirs()?.data_local_dir().join("storage").to_owned())
}

/// The blockchain data storage path.
///
/// Default: <base_path>/data
pub fn data_path(base_path: &Option<PathBuf>, data_path: &Option<PathBuf>) -> Result<PathBuf, PathError> {
	// custom?
	if let Some(data_path) = data_path {
		return Ok(data_path.clone())
	}

	// base
	if let Some(base_path) = base_path {
		return Ok(base_path.join("data"))
	}

	// local app data
	Ok(project_dirs()?.data_local_dir().join("data").to_owned())
}

/// The log files path.
///
/// Default: <base_path>/log
pub fn log_path(base_path: &Option<PathBuf>, log_path: &Option<PathBuf>) -> Result<PathBuf, PathError> {
	// custom?
	if let Some(log_path) = log_path {
		return Ok(log_path.clone())
	}

	// base
	if let Some(base_path) = base_path {
		return Ok(base_path.join("log"))
	}

	// log
	Ok(project_dirs()?.data_local_dir().join("log").to_owned())
}

/// The log files path.
pub fn config_path(base_path: &Option<PathBuf>, config_path: &Option<PathBuf>) -> Result<PathBuf, PathError> {
	// custom?
	if let Some(config_path) = config_path {
		return Ok(config_path.clone())
	}

	// base
	if let Some(base_path) = base_path {
		return Ok(base_path.join("etc"))
	}

	// log
	Ok(project_dirs()?.config_local_dir().to_owned())
}

pub async fn create_folder(path: PathBuf) -> std::io::Result<PathBuf> {
	match create_dir_all(&path).await {
		Ok(_) => Ok(path),
		Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(path),
		Err(e) => Err(e),
	}
}
