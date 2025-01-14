use co_messaging::MatrixEvent;
use std::{fs::File, io::Write};

#[test]
fn test_schemars() {
	let schema = schemars::schema_for!(MatrixEvent);
	let mut file = File::create("test-output/schema.json").expect("new file");
	file.write_all(serde_json::to_string_pretty(&schema).expect("json").as_bytes())
		.expect("file written");
	println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
