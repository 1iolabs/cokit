use anyhow::anyhow;
use co_core_co::CoAction;
use co_core_keystore::KeyStore;
use co_sdk::{state, CreateCo, Identity, SharedCoJoin, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE};
use helper::instance::Instance;

pub mod helper;

#[tokio::test]
async fn test_co_shared_join() {
	let mut peer1 = Instance::new(1).await;
	peer1.application.create_network(false).await.unwrap();
	let mut peer2 = Instance::new(1).await;
	peer2.application.create_network(false).await.unwrap();

	// networks
	let network1 = peer1.application.network().unwrap();
	let network2 = peer2.application.network().unwrap();

	// connect
	network2
		.dail(network1.peer_id(), network1.listeners().await.unwrap())
		.await
		.unwrap();
	network1
		.dail(network2.peer_id(), network2.listeners().await.unwrap())
		.await
		.unwrap();

	// create identity
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// peer1: create shared co
	let shared_co = tracing::trace_span!("peer1: created shared co", peer = ?network1.peer_id())
		.in_scope(|| async {
			peer1
				.application
				.create_co(
					identity1.clone(),
					CreateCo { id: "shared".into(), algorithm: None, name: "shared".to_owned() },
				)
				.await
				.unwrap()
		})
		.await;

	// peer1: add other peer identity
	tracing::trace_span!("peer1: added other peer identity", peer = ?network1.peer_id())
		.in_scope(|| async {
			shared_co
				.push(
					&identity1,
					CO_CORE_NAME_CO,
					&CoAction::ParticipantInvite {
						participant: identity2.identity().to_owned(),
						tags: Default::default(),
					},
				)
				.await
				.unwrap();
			shared_co
				.push(
					&identity1,
					CO_CORE_NAME_CO,
					&CoAction::ParticipantJoin { participant: identity2.identity().to_owned() },
				)
				.await
				.unwrap();
		})
		.await;

	// peer2: join
	tracing::trace_span!("peer2: join", peer = ?network2.peer_id())
		.in_scope(|| async {
			let (shared_co_state, shared_co_heads) = shared_co.external_reducer_state().await;
			SharedCoJoin::new(shared_co.id().clone())
				.with_trusted_peer(peer1.application.network().map(|network| network.peer_id()).unwrap())
				.with_heads(shared_co_heads, shared_co_state)
				.join(
					peer2.application.runtime_pool(),
					Some(peer2.application.network().unwrap().spawner()),
					peer2.application.storage(),
					peer2.application.local_co_reducer().await.unwrap(),
					identity2,
				)
				.await
				.unwrap();
		})
		.await;

	// peer2: get shared co
	let shared_co_2 = peer2.application.co_reducer(shared_co.id()).await.unwrap().unwrap();
	assert_eq!(shared_co.reducer_state().await, shared_co_2.reducer_state().await);
}

#[tokio::test]
async fn test_co_shared_join_encrypted() {
	let mut peer1 = Instance::new(1).await;
	peer1.application.create_network(false).await.unwrap();
	let mut peer2 = Instance::new(1).await;
	peer2.application.create_network(false).await.unwrap();

	// networks
	let network1 = peer1.application.network().unwrap();
	let network2 = peer2.application.network().unwrap();

	// connect
	network2
		.dail(network1.peer_id(), network1.listeners().await.unwrap())
		.await
		.unwrap();
	network1
		.dail(network2.peer_id(), network2.listeners().await.unwrap())
		.await
		.unwrap();

	// create identity
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// // listen
	// network1.did_discovery_subscribe(identity1.clone()).await.unwrap();
	// network2.did_discovery_subscribe(identity2.clone()).await.unwrap();

	// peer1: create shared co
	let shared_co = tracing::trace_span!("peer1: created shared co", peer = ?network1.peer_id())
		.in_scope(|| async {
			peer1
				.application
				.create_co(
					identity1.clone(),
					CreateCo { id: "shared".into(), algorithm: Some(Default::default()), name: "shared".to_owned() },
				)
				.await
				.unwrap()
		})
		.await;

	// peer1: add other peer identity
	tracing::trace_span!("peer1: added other peer identity", peer = ?network1.peer_id())
		.in_scope(|| async {
			shared_co
				.push(
					&identity1,
					CO_CORE_NAME_CO,
					&CoAction::ParticipantInvite {
						participant: identity2.identity().to_owned(),
						tags: Default::default(),
					},
				)
				.await
				.unwrap();
			shared_co
				.push(
					&identity1,
					CO_CORE_NAME_CO,
					&CoAction::ParticipantJoin { participant: identity2.identity().to_owned() },
				)
				.await
				.unwrap();
		})
		.await;

	// peer1: extract key
	let shared_co_secret = tracing::trace_span!("peer1: added other peer identity", peer = ?network1.peer_id())
		.in_scope(|| async {
			let local_co = peer1.application.local_co_reducer().await.unwrap();
			let key_store: KeyStore = local_co.state(CO_CORE_NAME_KEYSTORE).await.unwrap();
			let co = shared_co.co().await.unwrap();
			let key_reference = co.keys.as_ref().and_then(|keys| keys.first().map(|key| &key.id)).unwrap();
			let (_, key) = state::find(&local_co.storage(), &key_store.keys, |(k, _)| k == key_reference)
				.await
				.unwrap()
				.unwrap();
			let shared_co_secret = match key.secret {
				co_core_keystore::Secret::SharedKey(sec) => Ok(sec),
				_ => Err(anyhow!("Invalid secret")),
			}
			.unwrap();
			shared_co_secret
		})
		.await;

	// peer2: join
	tracing::trace_span!("peer2: join", peer = ?network2.peer_id())
		.in_scope(|| async {
			let (shared_co_state, shared_co_heads) = shared_co.external_reducer_state().await;
			SharedCoJoin::new(shared_co.id().clone())
				.with_trusted_peer(peer1.application.network().map(|network| network.peer_id()).unwrap())
				.with_encryption(shared_co_secret.into())
				.with_heads(shared_co_heads, shared_co_state)
				.join(
					peer2.application.runtime_pool(),
					Some(peer2.application.network().unwrap().spawner()),
					peer2.application.storage(),
					peer2.application.local_co_reducer().await.unwrap(),
					identity2,
				)
				.await
				.unwrap();
		})
		.await;

	// peer2: get shared co
	let shared_co_2 = peer2.application.co_reducer(shared_co.id()).await.unwrap().unwrap();
	assert_eq!(shared_co.reducer_state().await, shared_co_2.reducer_state().await);
}
