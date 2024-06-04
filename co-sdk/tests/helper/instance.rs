use co_sdk::{Application, ApplicationBuilder, DidKeyIdentity, DidKeyProvider, CO_CORE_NAME_KEYSTORE};
use std::env::current_dir;
use tracing::{level_filters::LevelFilter, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, Registry};

pub struct Instance {
	pub application: Application,
}
impl Instance {
	pub async fn new(instance: u8) -> Self {
		let identifier = format!("network-test-{}", instance);
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

	pub fn init_log() {
		let log_path = current_dir().unwrap().join("../data/log/co.log");
		std::fs::create_dir_all(log_path.parent().unwrap()).unwrap();
		let log_file = std::fs::File::options().append(true).create(true).open(log_path).unwrap();
		// let formatting_layer = BunyanFormattingLayer::new("co-daemon".into(), std::io::stdout);
		let formatting_layer = BunyanFormattingLayer::new("test".to_owned(), log_file);
		let subscriber = Registry::default()
			.with(LevelFilter::TRACE)
			.with(JsonStorageLayer)
			.with(formatting_layer);
		set_global_default(subscriber).ok();
		LogTracer::init().ok();
	}
}
