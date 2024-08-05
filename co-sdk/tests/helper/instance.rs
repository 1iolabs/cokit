use co_sdk::{Application, ApplicationBuilder, DidKeyIdentity, DidKeyProvider, TracingBuilder, CO_CORE_NAME_KEYSTORE};

pub struct Instance {
	pub application: Application,
}
impl Instance {
	pub async fn new(instance: u8) -> Self {
		let identifier = format!("network-test-{}", instance);

		// log
		TracingBuilder::new("test".into(), None)
			//.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			//.with_open_telemetry("http://localhost:4317")
			.with_stderr_logging()
			.init()
			.ok();

		// app
		let builder = ApplicationBuilder::new_memory(identifier);
		let application = builder.without_keychain().build().await.expect("application");
		Self { application }
	}

	/// Create `did:key` identity and store it to local co keystore.
	pub async fn create_identity(&self) -> DidKeyIdentity {
		let identity = DidKeyIdentity::generate(None);
		let co = self.application.local_co_reducer().await.unwrap();
		let provider = DidKeyProvider::new(co, CO_CORE_NAME_KEYSTORE);
		provider.store(&identity, None).await.unwrap();
		identity
	}
}
