// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
