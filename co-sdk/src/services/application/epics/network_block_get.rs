use crate::{
	library::{
		connections_peer_provider::ConnectionsPeerProvider, find_co_secret::find_co_secret_by_membership,
		network_identity::network_identity_by_id, network_queue::TaskState, settings_timeout::settings_timeout,
	},
	services::application::NetworkBlockGetAction,
	Action, CoContext, CoNetworkTaskSpawner, CoReducerFactory, CoToken, CoTokenParameters, ConnectionMessage,
	CO_ID_LOCAL,
};
use cid::Cid;
use co_actor::{ActionDispatch, Actions, ActorHandle};
use co_identity::Identity;
use co_network::{
	backoff_with_jitter,
	bitswap::{GetNetworkTask, Token},
	PeerProvider,
};
use co_primitives::{BlockSerializer, CoId};
use co_storage::StorageError;
use futures::{future::Either, pin_mut, stream, FutureExt, Stream, StreamExt};
use std::time::Duration;

const NETWORK_QUEUE_TYPE: &str = "network-block-get";

/// Request block from network.
///
/// In: [`Action::NetworkBlockGet`]
/// Out: [`Action::NetworkBlockGetComplete`] | [`Action::NetworkTaskQueue`]
pub fn network_block_get(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkBlockGet(action) => {
			let action = action.clone();
			let context = context.clone();
			let actions = actions.clone();
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
					Either::Right(handle_network_block_get(
						context.clone(),
						network,
						connections,
						actions,
						action.clone(),
					))
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
	actions: &Actions<Action, (), CoContext>,
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
			let actions = actions.clone();
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
						handle_network_block_get(context.clone(), network, connections, actions, action.clone())
							.flat_map(move |item| match item {
								Ok(Action::NetworkBlockGetComplete(request, result)) => stream::iter(vec![
									Ok(Action::NetworkTaskExecuteComplete {
										co: action.co.clone(),
										task_id: task_id.clone(),
										task_state: match result {
											Ok(_) => TaskState::Done,
											Err(_) => TaskState::Failed,
										},
									}),
									Ok(Action::NetworkBlockGetComplete(request, result)),
								]),
								item => stream::iter(vec![item]),
							}),
					)
				}
				.into_stream()
				.flatten(),
			)
		},
		_ => None,
	}
}

/// Handle `NetworkBlockGetAction`.
///
/// Note:
/// - We are not allowed to access the reducer state here because of deadlocks.
/// - When the reducer is initially loaded but has no blocks locally it will fetch them from network.
fn handle_network_block_get(
	context: CoContext,
	network: CoNetworkTaskSpawner,
	connections: ActorHandle<ConnectionMessage>,
	actions: Actions<Action, (), CoContext>,
	action: NetworkBlockGetAction,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	ActionDispatch::execute_with_response(
		actions,
		context.tasks(),
		{
			let action = action.clone();
			move |_dispatch| async move {
				let identity = network_identity_by_id(&context, &action.parent_co, &action.co, None).await?;
				let peer_provider =
					ConnectionsPeerProvider::new(action.co.clone(), identity.identity().to_owned(), connections);
				let token = network_token(&context, &network, &action.parent_co, &action.co).await?;
				let timeout = settings_timeout(&context, &CoId::from(CO_ID_LOCAL), Some("block-get")).await;
				let concurrent = 10;

				// execute
				get_network(network, peer_provider, vec![token.to_bitswap_token()?], timeout, concurrent, action.cid)
					.await?;

				// result
				Ok(())
			}
		},
		move |result| Action::NetworkBlockGetComplete(action, result),
	)
}

/// Create a CoToken for the co.
async fn network_token(
	context: &CoContext,
	network: &CoNetworkTaskSpawner,
	parent_co_id: &CoId,
	co_id: &CoId,
) -> Result<CoToken, anyhow::Error> {
	let parent = context.try_co_reducer(parent_co_id).await?;
	let secret = find_co_secret_by_membership(&parent, co_id).await?;
	let token = if let Some(shared_secret) = secret {
		CoToken::new(&shared_secret, CoTokenParameters(network.local_peer_id(), co_id.clone()))?
	} else {
		CoToken::new_unsigned(CoTokenParameters(network.local_peer_id(), co_id.clone()))
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
	cid: Cid,
) -> Result<(), StorageError> {
	let deadline = tokio::time::Instant::now() + timeout;
	let mut retry = 1;
	loop {
		// start network task for every peer.
		let get_stream = peer_provider
			.peers_added()
			.flat_map(stream::iter)
			.map(|peer| GetNetworkTask::get(&spawner, cid, tokens.clone(), [peer].into()))
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
					tracing::warn!(?err, ?cid, "get-network-failed");
				},
				// timeout
				Err(err) => {
					return Err(StorageError::NotFound(cid, err.into()));
				},
			}
		}

		// timeout?
		if tokio::time::Instant::now() > deadline {
			break;
		}

		// backoff
		tokio::time::sleep(backoff_with_jitter(retry)).await;

		// retry
		retry += 1;
	}
	Err(StorageError::NotFound(cid, anyhow::anyhow!("Insufficent peers")))
}
