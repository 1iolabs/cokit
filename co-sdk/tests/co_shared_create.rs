use co_core_co::CoAction;
use co_sdk::{tags, CreateCo, CO_CORE_NAME_CO};
use helper::instance::Instance;

pub mod helper;

#[tokio::test]
async fn test_co_shared_create() {
	let peer1 = Instance::new(1).await;

	// create identity
	let identity = peer1.create_identity().await;

	// create shared co
	let shared_co = peer1
		.application
		.create_co(
			identity.clone(),
			CreateCo { id: "shared".into(), algorithm: Some(Default::default()), name: "shared".to_owned() },
		)
		.await
		.unwrap();

	// push
	shared_co
		.push(&identity, CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("hello": "world") })
		.await
		.unwrap();
}
