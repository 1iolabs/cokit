use co_sdk::{CreateCo, DidKeyIdentity, DidKeyProvider, CO_CORE_NAME_KEYSTORE};
use helper::instance::Instance;

mod helper;

#[tokio::test]
async fn create_shared_co_test() {
	let mut peer1 = Instance::new(1).await;
	peer1.application.create_network(false).await.unwrap();

	// create identity
	let identity = DidKeyIdentity::generate(None);
	let co = peer1.application.local_co_reducer().await.unwrap();
	let provider = DidKeyProvider::new(co, CO_CORE_NAME_KEYSTORE);
	provider.store(&identity, None).await.unwrap();

	// create shared co
	peer1
		.application
		.create_co(
			identity.clone(),
			CreateCo { id: "shared".into(), algorithm: Some(Default::default()), name: "shared".to_owned() },
		)
		.await
		.unwrap();

	// shutdown
	// peer1.application.shutdown_application().await;

	// let mut peer2 = Instance::new(2).await;
	// peer2.application.create_network(false).await.unwrap();
}
