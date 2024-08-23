use co_sdk::{Application, ApplicationBuilder, DidKeyIdentity, DidKeyProvider, TracingBuilder, CO_CORE_NAME_KEYSTORE};
use tracing::subscriber::DefaultGuard;

pub struct Instances {
	next_instance_id: u8,
	_guard: Option<DefaultGuard>,
}
impl Instances {
	pub fn new(name: impl Into<String>) -> Self {
		// log
		let _guard = TracingBuilder::new(name.into(), None)
			//.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			//.with_open_telemetry("http://localhost:4317")
			.with_stderr_logging()
			.with_env_filter_directives(
				"trace,log=warn,quinn_proto=warn,hickory_proto=warn,co_storage::storage::memory=warn",
			)
			.unwrap()
			//.with_env_filter_directives("info,co_sdk=trace,co_network=trace")
			.init()
			.ok();
		Self { next_instance_id: 1, _guard: None }
	}

	pub async fn create(&mut self) -> Instance {
		let instance_id = self.next_instance_id;
		self.next_instance_id += 1;
		Instance::new(instance_id).await
	}
}

pub struct Instance {
	pub application: Application,
}
impl Instance {
	pub async fn new(instance: u8) -> Self {
		let identifier = format!("network-test-{}", instance);

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
