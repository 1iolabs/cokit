use co_sdk::{Application, ApplicationBuilder};
use std::env::current_dir;

pub struct Instance {
	pub application: Application,
}
impl Instance {
	pub async fn new(instance: u8) -> Self {
		let identifier = format!("network-test-{}", instance);
		let builder = ApplicationBuilder::new_memory(identifier);
		let application = builder
			.without_keychain()
			.with_bunyan_logging(Some(current_dir().unwrap().join("../data/log/co.log")))
			.build()
			.await
			.expect("application");
		Self { application }
	}
}
