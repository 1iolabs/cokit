// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

const COMMANDS: &[&str] = &[
	"get_co_state",
	"push_action",
	"resolve_cid",
	"storage_get",
	"storage_set",
	"get_actions",
	"create_identity",
	"session_open",
	"session_close",
	"create_co",
];

fn main() {
	tauri_plugin::Builder::new(COMMANDS)
		.android_path("android")
		.ios_path("ios")
		.build();
}
