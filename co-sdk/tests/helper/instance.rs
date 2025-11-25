use co_network::NetworkApi;
use co_sdk::{
	Application, ApplicationBuilder, DidKeyIdentity, DidKeyProvider, NetworkSettings, TracingBuilder,
	CO_CORE_NAME_KEYSTORE,
};
use tracing::subscriber::DefaultGuard;

pub struct Instances {
	next_instance_id: u8,
	_guard: Option<DefaultGuard>,
}
impl Instances {
	pub fn new(name: impl Into<String>) -> Self {
		// log
		let _guard = TracingBuilder::new(name.into(), None)
			.with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			//.with_open_telemetry("http://localhost:4317")
			//.with_stderr_logging()
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

	pub async fn networking(
		peer1: &mut Instance,
		peer2: &mut Instance,
		dail_peer1_to_peer2: bool,
		dail_peer2_to_peer1: bool,
	) -> (NetworkApi, NetworkApi) {
		// start
		peer1
			.application
			.create_network(NetworkSettings::default().with_localhost())
			.await
			.unwrap();
		peer2
			.application
			.create_network(NetworkSettings::default().with_localhost())
			.await
			.unwrap();

		// networks
		let network1 = peer1.application.context().network().await.unwrap();
		let network2 = peer2.application.context().network().await.unwrap();

		// connect
		//  because of localhost we need to explicitly dial (no mDNS on localhost).
		if dail_peer2_to_peer1 {
			network2
				.dial(
					Some(network1.local_peer_id()),
					network1.listeners(true, false).await.unwrap().into_iter().collect(),
				)
				.await
				.unwrap();
		}
		if dail_peer1_to_peer2 {
			network1
				.dial(
					Some(network2.local_peer_id()),
					network2.listeners(true, false).await.unwrap().into_iter().collect(),
				)
				.await
				.unwrap();
		}

		// result
		(network1, network2)
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
