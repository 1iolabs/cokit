const COMMANDS: &[&str] = &["get_co_state", "push_action", "resolve_cid", "storage_get", "storage_set"];

fn main() {
	tauri_plugin::Builder::new(COMMANDS)
		.android_path("android")
		.ios_path("ios")
		.build();
}
