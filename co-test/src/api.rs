// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::TmpDir;
use std::path::PathBuf;

pub fn test_application_identifier(test_name: &str) -> String {
	let application_identifier = format!("{}-{}", test_name, uuid::Uuid::new_v4());
	application_identifier
}

pub fn test_tmp_dir() -> TmpDir {
	let tmp = TmpDir::new("co");
	println!("path: {:?}", tmp.path());
	tmp
}

pub fn test_repository_path() -> PathBuf {
	std::env::current_exe()
		.unwrap()
		.parent()
		.unwrap()
		.join("../../..") // "target/debug/deps"
		.canonicalize()
		.unwrap()
}

pub fn test_log_path() -> PathBuf {
	let log_path = test_repository_path().join("data/log/co.log");
	println!("log_path: {:?}", log_path);
	log_path
}
