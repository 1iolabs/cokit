use crate::{
	find_co_secret,
	library::{
		connections_peer_provider::ConnectionsPeerProvider, network_identity::network_identity,
		network_queue::TaskState, settings_timeout::settings_timeout, to_external_cid::to_external_mapped,
	},
	services::application::NetworkBlockGetAction,
	Action, CoContext, CoNetworkTaskSpawner, CoReducer, CoReducerFactory, CoToken, CoTokenParameters,
	ConnectionMessage,
};
use co_actor::{Actions, ActorHandle};
use co_identity::Identity;
use co_network::{
	backoff_with_jitter,
	bitswap::{GetNetworkTask, Token},
	PeerProvider,
};
use co_primitives::{BlockSerializer, OptionMappedCid};
use co_storage::StorageError;
use futures::{future::Either, pin_mut, stream, FutureExt, Stream, StreamExt};
use std::time::Duration;

const NETWORK_QUEUE_TYPE: &str = "network-block-get";

/// Request block from network.
///
/// In: [`Action::NetworkBlockGet`]
/// Out: [`Action::NetworkBlockGetComplete`] | [`Action::NetworkTaskQueue`]
pub fn network_block_get(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkBlockGet(action) => {
			let action = action.clone();
			let context = context.clone();
			Some(
				async move {
					// network
					let network = context.network().await;
					let Some((network, connections)) = network else {
						return Either::Left(stream::iter([Action::network_task_queue(
							action.co.clone(),
							action.cid.to_string(),
							NETWORK_QUEUE_TYPE,
							format!("Get block {} co:{}", action.cid, action.co),
							&action,
						)]));
					};

					// send
					Either::Right(handle_network_block_get(context.clone(), network, connections, action.clone()))
				}
				.into_stream()
				.flatten(),
			)
		},
		_ => None,
	}
}

/// Execute queued [`NetworkBlockGetAction`].
///
/// In: [`Action::NetworkTaskExecute`]
/// Out: [`Action::NetworkBlockGetComplete`], [`Action::NetworkTaskExecuteComplete`]
pub fn network_task_execute(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkTaskExecute { co, task_id, task_type, task } if task_type == NETWORK_QUEUE_TYPE => {
			let action = BlockSerializer::default().deserialize::<NetworkBlockGetAction>(task);
			let context = context.clone();
			let task_id = task_id.clone();
			let co = co.clone();
			Some(
				async move {
					// action
					let Ok(action) = action else {
						return Either::Left(stream::iter([Ok(Action::NetworkTaskExecuteComplete {
							co,
							task_id,
							task_state: TaskState::Failed,
						})]));
					};

					// network
					let network = context.network().await;
					let Some((network, connections)) = network else {
						return Either::Left(stream::iter([Ok(Action::NetworkTaskExecuteComplete {
							co,
							task_id,
							task_state: TaskState::Failed,
						})]));
					};

					// send
					Either::Right(
						handle_network_block_get(context.clone(), network, connections, action.clone()).flat_map(
							move |result| {
								let task_complete = match &result {
									Ok(_) => Action::NetworkTaskExecuteComplete {
										co: action.co.clone(),
										task_id: task_id.clone(),
										task_state: TaskState::Done,
									},
									Err(_) => Action::NetworkTaskExecuteComplete {
										co: action.co.clone(),
										task_id: task_id.clone(),
										task_state: TaskState::Failed,
									},
								};
								stream::iter([result, Ok(task_complete)])
							},
						),
					)
				}
				.into_stream()
				.flatten(),
			)
		},
		_ => None,
	}
}

fn handle_network_block_get(
	context: CoContext,
	network: CoNetworkTaskSpawner,
	connections: ActorHandle<ConnectionMessage>,
	action: NetworkBlockGetAction,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	async move {
		let co = context.try_co_reducer(&action.co).await?;
		let storage = co.storage();
		let identity = network_identity(&context, &co, None).await?;
		let peer_provider =
			ConnectionsPeerProvider::new(action.co.clone(), identity.identity().to_owned(), connections);
		let token = network_token(&context, &network, &co).await?;
		let timeout = settings_timeout(&context, co.id(), Some("block-get")).await;
		let concurrent = 10;
		let mapped = to_external_mapped(&storage, action.cid).await;

		// execute
		let result =
			get_network(network, peer_provider, vec![token.to_bitswap_token()?], timeout, concurrent, mapped).await;

		// result
		Ok(Action::NetworkBlockGetComplete(action, result))
	}
	.into_stream()
}

/// Create a CoToken for the co.
async fn network_token(
	context: &CoContext,
	network: &CoNetworkTaskSpawner,
	co: &CoReducer,
) -> Result<CoToken, anyhow::Error> {
	let parent_id = co.parent_id().ok_or_else(|| anyhow::anyhow!("No parent co: {}", co.id()))?;
	let parent = context.try_co_reducer(parent_id).await?;
	let secret = find_co_secret(&parent, co).await?;
	let token = if let Some(shared_secret) = secret {
		CoToken::new(&shared_secret, CoTokenParameters(network.local_peer_id(), co.id().clone()))?
	} else {
		CoToken::new_unsigned(CoTokenParameters(network.local_peer_id(), co.id().clone()))
	};
	Ok(token)
}

/// Get block from co network.
/// Ask `concurrent` peers in parallel for a block.
/// Use the first block that will be received.
///
/// ```mermaid
/// sequenceDiagram
/// 		participant A as Caller
/// 		participant S as Storage
/// 		participant C as Connections
/// 		participant B as Bitswap
/// 		A ->> S: get(cid_a)
/// 		activate A
/// 		loop
/// 			S ->>+ C: connect (use via peers_added)
/// 			loop with concurrency
/// 				C -->+ S: peer
/// 				S ->>+ B: get `cid_a` from `peer`
/// 				B -->- S: Result<(), Error>
/// 				alt ok
/// 					S ->> A: Ok
/// 					deactivate A
/// 				else
/// 					S -->- S: Error
/// 				end
/// 			end
/// 			C --x- S: close
/// 			opt timeout
/// 				S --x A: Err(Insufficient peers)
/// 			end
/// 		end
/// ```
#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(peer_provider))]
async fn get_network(
	spawner: CoNetworkTaskSpawner,
	peer_provider: impl PeerProvider,
	tokens: Vec<Token>,
	timeout: Duration,
	concurrent: usize,
	mapped: OptionMappedCid,
) -> Result<(), StorageError> {
	let deadline = tokio::time::Instant::now() + timeout;
	let mut retry = 1;
	loop {
		// start network task for every peer.
		let get_stream = peer_provider
			.peers_added()
			.flat_map(stream::iter)
			.map(|peer| GetNetworkTask::get(&spawner, mapped.external(), tokens.clone(), [peer].into()))
			.buffer_unordered(concurrent);
		pin_mut!(get_stream);
		loop {
			let result = tokio::time::timeout_at(deadline, get_stream.next()).await;
			match result {
				// no more peers
				Ok(None) => {
					break;
				},
				// some `GetNetworkTask` reported ok
				Ok(Some(Ok(()))) => {
					// done
					return Ok(());
				},
				// some `GetNetworkTask` reported a error
				Ok(Some(Err(err))) => {
					// log
					tracing::warn!(?err, ?mapped, "get-network-failed");
				},
				// timeout
				Err(err) => {
					return Err(StorageError::NotFound(mapped.internal(), err.into()));
				},
			}
		}

		// backoff
		tokio::time::sleep(backoff_with_jitter(retry)).await;

		// timeout?
		if tokio::time::Instant::now() > deadline {
			break;
		}

		// retry
		retry += 1;
	}
	Err(StorageError::NotFound(mapped.internal(), anyhow::anyhow!("Insufficent peers")))
}
