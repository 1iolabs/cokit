// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_network::NetworkApi;
use co_sdk::{
	Application, ApplicationBuilder, DidKeyIdentity, DidKeyProvider, Identity, NetworkSettings, CO_CORE_NAME_KEYSTORE,
};

pub struct Instances {
	next_instance_id: u8,
}
impl Instances {
	pub fn new(name: impl Into<String>) -> Self {
		let _ = name;
		co_test::init_test_log();
		Self { next_instance_id: 1 }
	}

	/// Create a new peer.
	pub async fn create(&mut self) -> Instance {
		let instance_id = self.next_instance_id;
		self.next_instance_id += 1;
		Instance::new(instance_id).await
	}

	/// Create a new peer.
	pub async fn create_builder(&mut self, build: impl FnOnce(ApplicationBuilder) -> ApplicationBuilder) -> Instance {
		let instance_id = self.next_instance_id;
		self.next_instance_id += 1;
		Instance::new_builder(instance_id, build).await
	}

	/// Start networking for two peers and optionally dial them.
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
		Self::new_builder(instance, |builder| builder).await
	}

	pub async fn new_builder(instance: u8, build: impl FnOnce(ApplicationBuilder) -> ApplicationBuilder) -> Self {
		let identifier = format!("network-test-{}", instance);

		// app
		let builder = ApplicationBuilder::new_memory(identifier);
		let application = build(builder).without_keychain().build().await.expect("application");
		Self { application }
	}

	/// Create `did:key` identity and store it to local co keystore.
	pub async fn create_identity(&self) -> DidKeyIdentity {
		let identity = DidKeyIdentity::generate(None);
		let co = self.application.local_co_reducer().await.unwrap();
		let provider = DidKeyProvider::new(co, CO_CORE_NAME_KEYSTORE);
		provider.store(&identity, None).await.unwrap();
		tracing::trace!(
			did = identity.identity(),
			application = self.application.settings().identifier,
			"create-identity"
		);
		identity
	}
}
