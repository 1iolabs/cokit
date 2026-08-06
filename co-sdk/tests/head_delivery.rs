// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#![cfg(all(feature = "guard", feature = "network"))]

use async_trait::async_trait;
use co_core_board::{Board, BoardAction, Task, TaskLock, TaskTransition};
use co_core_co::CoAction;
use co_core_membership::MembershipState;
use co_guard::AccessGuard;
use co_log::EntryBlock;
use co_network::{
	connections::{ConnectionAction, ConnectionMessage, ReleaseAction},
	NetworkApi,
};
use co_primitives::{
	Block, BlockSerializer, CoTryStreamExt, KnownMultiCodec, Network, NetworkPeer, TagsAction, TagsExpr,
};
use co_sdk::{
	join_unrelated_co, push_heads_to_dids, tags, Action, Application, ApplicationBuilder, CoConnectivity, CoId,
	CoReducer, CoreName, CreateCo, Did, DidKeyIdentity, DidKeyProvider, HeadsDeliveryCompleteAction,
	HeadsDeliveryOutcome, HeadsDeliveryPhase, HeadsRecipient, Identity, MonotonicCoUuid, PrivateIdentity,
	CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE, CO_ID_LOCAL,
};
use co_storage::BlockStorage;
use co_test::test_tmp_dir;
use futures::{future::ready, Stream, StreamExt, TryStreamExt};
use helper::{instance::Instances, shared_co::SharedCo};
use std::{
	collections::BTreeSet,
	sync::{
		atomic::{AtomicU8, AtomicUsize, Ordering},
		Arc,
	},
	time::Duration,
};
use tokio::{sync::Notify, time::timeout};

pub mod helper;

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
enum GuardMode {
	Allow = 0,
	Deny = 1,
	Error = 2,
}

#[derive(Clone)]
struct MutableGuard(Arc<AtomicU8>);

impl MutableGuard {
	fn new(mode: GuardMode) -> Self {
		Self(Arc::new(AtomicU8::new(mode as u8)))
	}

	#[allow(dead_code)]
	fn set(&self, mode: GuardMode) {
		self.0.store(mode as u8, Ordering::SeqCst);
	}
}

#[async_trait]
impl AccessGuard for MutableGuard {
	async fn check_access(&self, _co: &CoId, _requester: &str) -> Result<bool, anyhow::Error> {
		match self.0.load(Ordering::SeqCst) {
			0 => Ok(true),
			1 => Ok(false),
			2 => Err(anyhow::anyhow!("guard unavailable")),
			value => Err(anyhow::anyhow!("invalid guard mode {value}")),
		}
	}
}

#[derive(Clone)]
struct CountingGuard {
	mode: MutableGuard,
	calls: Arc<AtomicUsize>,
}

impl CountingGuard {
	fn new(mode: GuardMode) -> Self {
		Self { mode: MutableGuard::new(mode), calls: Arc::new(AtomicUsize::new(0)) }
	}

	fn set(&self, mode: GuardMode) {
		self.mode.set(mode);
	}

	fn reset(&self) {
		self.calls.store(0, Ordering::SeqCst);
	}

	fn calls(&self) -> usize {
		self.calls.load(Ordering::SeqCst)
	}
}

#[async_trait]
impl AccessGuard for CountingGuard {
	async fn check_access(&self, co: &CoId, requester: &str) -> Result<bool, anyhow::Error> {
		self.calls.fetch_add(1, Ordering::SeqCst);
		self.mode.check_access(co, requester).await
	}
}

#[derive(serde::Serialize)]
struct SerializedHeadsDeliveryIntent {
	co: CoId,
	from: Did,
	recipient: Did,
	connectivity: CoConnectivity,
}

fn network_queue_core() -> CoreName<'static, Board> {
	CoreName::new("network_queue")
}

#[derive(Clone, Default)]
struct StaleDenyGuard {
	calls: Arc<AtomicUsize>,
	first_check_started: Arc<Notify>,
	release_first_check: Arc<Notify>,
}

#[async_trait]
impl AccessGuard for StaleDenyGuard {
	async fn check_access(&self, _co: &CoId, _requester: &str) -> Result<bool, anyhow::Error> {
		if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
			self.first_check_started.notify_one();
			self.release_first_check.notified().await;
			Ok(false)
		} else {
			Ok(true)
		}
	}
}

#[derive(Clone)]
struct GatedGuard {
	mode: MutableGuard,
	calls: Arc<AtomicUsize>,
	first_check_started: Arc<Notify>,
	release_first_check: Arc<Notify>,
}

impl GatedGuard {
	fn new(mode: GuardMode) -> Self {
		Self {
			mode: MutableGuard::new(mode),
			calls: Arc::new(AtomicUsize::new(0)),
			first_check_started: Arc::new(Notify::new()),
			release_first_check: Arc::new(Notify::new()),
		}
	}

	fn set(&self, mode: GuardMode) {
		self.mode.set(mode);
	}

	fn release(&self) {
		self.release_first_check.notify_one();
	}
}

#[async_trait]
impl AccessGuard for GatedGuard {
	async fn check_access(&self, co: &CoId, requester: &str) -> Result<bool, anyhow::Error> {
		if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
			self.first_check_started.notify_one();
			self.release_first_check.notified().await;
		}
		self.mode.check_access(co, requester).await
	}
}

async fn removed_participant_fixture(guard: MutableGuard) -> SharedCo {
	let mut instances = Instances::new("targeted-heads");
	let owner = instances.create_builder(move |builder| builder.with_access_guard(guard)).await;
	let recipient = instances.create().await;
	let shared = SharedCo::create_with_peers(owner, recipient, "targeted-heads").await;
	let (owner_co, owner_identity) = shared.reducer(0, "targeted-heads").await;
	let recipient_did = shared.identity(1).identity().to_owned();
	owner_co
		.push(
			&owner_identity,
			CO_CORE_NAME_CO,
			&CoAction::ParticipantRemove { participant: recipient_did, tags: Default::default() },
		)
		.await
		.unwrap();
	let owner = shared.application(0);
	owner
		.context()
		.network_connections()
		.await
		.unwrap()
		.dispatch(ConnectionMessage::Action(ConnectionAction::Release(ReleaseAction {
			id: CoId::from("targeted-heads"),
		})))
		.unwrap();
	let network = owner.context().network().await.unwrap();
	timeout(Duration::from_secs(10), async {
		loop {
			if network
				.co_overview(CoId::from("targeted-heads"))
				.await
				.unwrap()
				.peers
				.is_empty()
			{
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.unwrap();
	drain_bootstrap_head_deliveries(&owner, &CoId::from("targeted-heads")).await;
	shared
}

fn peer_connectivity(network: &NetworkApi) -> CoConnectivity {
	CoConnectivity {
		network: [Network::Peer(NetworkPeer { peer: network.local_peer_id().to_bytes(), addresses: vec![] })]
			.into_iter()
			.collect(),
		participants: BTreeSet::new(),
	}
}

async fn dispatch_and_wait_for_execution(shared: &SharedCo, from: Did, recipient: Did) -> HeadsDeliveryCompleteAction {
	let owner = shared.application(0);
	let recipient_network = shared.application(1).context().network().await.unwrap();
	let mut completions = Box::pin(owner.actions().filter_map({
		let recipient = recipient.clone();
		move |action| {
			let recipient = recipient.clone();
			async move {
				match action {
					Action::HeadsDeliveryComplete(done) if done.recipient == recipient => Some(done),
					_ => None,
				}
			}
		}
	}));
	push_heads_to_dids(
		owner.context(),
		CoId::from("targeted-heads"),
		from,
		[HeadsRecipient { did: recipient.clone(), connectivity: peer_connectivity(&recipient_network) }],
	)
	.unwrap();
	let admission = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(admission.phase, HeadsDeliveryPhase::Admission);
	assert_eq!(admission.attempted_heads, None);
	assert!(matches!(admission.outcome, HeadsDeliveryOutcome::Queued));
	unblock_preexisting_head_delivery(&owner, &CoId::from("targeted-heads"), &recipient).await;
	let execution = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(execution.phase, HeadsDeliveryPhase::Execution);
	execution
}

#[tokio::test]
async fn allowed_nonparticipant_receives_encrypted_heads() {
	let guard = MutableGuard::new(GuardMode::Allow);
	let shared = removed_participant_fixture(guard).await;
	let (owner_co, owner) = shared.reducer(0, "targeted-heads").await;
	let (recipient_co, _) = shared.reducer(1, "targeted-heads").await;
	let recipient = shared.identity(1).identity().to_owned();
	let recipient_network = shared.application(1).context().network().await.unwrap();
	let mut forwarded = Box::pin(shared.application(1).actions().filter_map(|action| async move {
		match action {
			Action::PushHeadsToDids(request) if request.co == CoId::from("targeted-heads") => Some(request),
			_ => None,
		}
	}));

	owner_co
		.push(&owner, CO_CORE_NAME_CO, &CoAction::Tags { action: TagsAction::insert(tags!("revision": "allowed")) })
		.await
		.unwrap();
	let expected = owner_co.reducer_state().await;
	let mut recipient_states = Box::pin(recipient_co.reducer_state_stream());
	let mut completions = Box::pin(shared.application(0).actions().filter_map({
		let recipient = recipient.clone();
		move |action| {
			let recipient = recipient.clone();
			async move {
				match action {
					Action::HeadsDeliveryComplete(done) if done.recipient == recipient => Some(done),
					_ => None,
				}
			}
		}
	}));

	push_heads_to_dids(
		shared.application(0).context(),
		CoId::from("targeted-heads"),
		owner.identity().to_owned(),
		[HeadsRecipient { did: recipient.clone(), connectivity: peer_connectivity(&recipient_network) }],
	)
	.unwrap();

	let admission = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(admission.phase, HeadsDeliveryPhase::Admission);
	assert_eq!(admission.attempted_heads, None);
	assert!(matches!(admission.outcome, HeadsDeliveryOutcome::Queued));
	unblock_preexisting_head_delivery(&shared.application(0), &CoId::from("targeted-heads"), &recipient).await;
	let done = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(done.phase, HeadsDeliveryPhase::Execution);
	assert!(matches!(done.outcome, HeadsDeliveryOutcome::Delivered { ref peers } if !peers.is_empty()));
	timeout(Duration::from_secs(10), async {
		while recipient_co.reducer_state().await != expected {
			recipient_states.next().await;
		}
	})
	.await
	.unwrap();
	assert!(
		timeout(Duration::from_millis(300), forwarded.next()).await.is_err(),
		"received heads must not schedule another outbound push"
	);
}

#[tokio::test]
async fn repeated_deliveries_release_removed_recipient_connection() {
	let shared = removed_participant_fixture(MutableGuard::new(GuardMode::Allow)).await;
	let (_, owner) = shared.reducer(0, "targeted-heads").await;
	let recipient = shared.identity(1).identity().to_owned();
	let network = shared.application(0).context().network().await.unwrap();
	timeout(Duration::from_secs(10), async {
		loop {
			if network
				.overview()
				.await
				.unwrap()
				.connections
				.dids
				.iter()
				.all(|entry| entry.to != recipient)
			{
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("completed SharedCo bootstrap must release its DID use");

	for _ in 0..2 {
		let done = dispatch_and_wait_for_execution(&shared, owner.identity().to_owned(), recipient.clone()).await;
		assert!(matches!(done.outcome, HeadsDeliveryOutcome::Delivered { ref peers } if !peers.is_empty()));
	}

	timeout(Duration::from_secs(10), async {
		loop {
			let overview = network.overview().await.unwrap();
			if overview.connections.dids.iter().all(|entry| entry.to != recipient) {
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("completed targeted deliveries must release their DID connection");
}

#[tokio::test]
async fn same_did_remote_peer_is_not_filtered_as_sender() {
	let mut instances = Instances::new("same-did-targeted-heads");
	let guard = MutableGuard::new(GuardMode::Allow);
	let mut device_a = instances.create_builder(move |builder| builder.with_access_guard(guard)).await;
	let mut device_b = instances.create().await;
	let owner = device_a.create_identity().await;
	let shared_identity = DidKeyIdentity::generate(None);
	let shared_did = shared_identity.identity().to_owned();
	let device_a_local = device_a.application.local_co_reducer().await.unwrap();
	DidKeyProvider::new(device_a_local, CO_CORE_NAME_KEYSTORE)
		.store(&shared_identity, None)
		.await
		.unwrap();
	let device_b_local = device_b.application.local_co_reducer().await.unwrap();
	DidKeyProvider::new(device_b_local, CO_CORE_NAME_KEYSTORE)
		.store(&shared_identity, None)
		.await
		.unwrap();

	let (device_a_network, device_b_network) = Instances::networking(&mut device_a, &mut device_b, true, true).await;
	let co = CoId::from("same-did-targeted-heads");
	let device_a_co = device_a
		.application
		.create_co(owner.clone(), CreateCo::new(co.clone(), None))
		.await
		.unwrap();
	device_a_co
		.push(
			&owner,
			CO_CORE_NAME_CO,
			&CoAction::ParticipantPending { participant: shared_did.clone(), tags: Default::default() },
		)
		.await
		.unwrap();
	device_a_co
		.push(
			&owner,
			CO_CORE_NAME_CO,
			&CoAction::ParticipantJoin { participant: shared_did.clone(), tags: Default::default() },
		)
		.await
		.unwrap();
	let shared_private = shared_identity.clone().boxed();
	let owner_public = owner.clone().boxed_public();
	let active = helper::shared_co::wait_membership_state(device_b.application.actions(), [MembershipState::Active]);
	let (joined, active) = futures::join!(
		join_unrelated_co(
			device_b.application.context(),
			&shared_private,
			&owner_public,
			co.clone(),
			peer_connectivity(&device_a_network),
		),
		timeout(Duration::from_secs(10), active),
	);
	joined.unwrap();
	let active = active.unwrap().unwrap();
	assert_eq!(active, (MembershipState::Active, co.clone(), shared_did.clone()));
	let device_b_co = device_b.application.co_reducer(co.clone()).await.unwrap().unwrap();
	assert_eq!(device_b_co.reducer_state().await, device_a_co.reducer_state().await);

	device_a_co
		.push(
			&owner,
			CO_CORE_NAME_CO,
			&CoAction::ParticipantRemove { participant: shared_did.clone(), tags: Default::default() },
		)
		.await
		.unwrap();
	let device_a_state = device_a_co.reducer_state().await;
	let active_participants = co_sdk::state::participants_active(&device_a_co.storage(), device_a_state.co())
		.await
		.unwrap();
	assert!(active_participants.iter().all(|participant| participant.did != shared_did));
	device_a
		.application
		.context()
		.network_connections()
		.await
		.unwrap()
		.dispatch(ConnectionMessage::Action(ConnectionAction::Release(ReleaseAction { id: co.clone() })))
		.unwrap();
	timeout(Duration::from_secs(10), async {
		loop {
			if device_a_network.co_overview(co.clone()).await.unwrap().peers.is_empty() {
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.unwrap();
	drain_bootstrap_head_deliveries(&device_a.application, &co).await;
	assert_eq!(head_task_count(&device_a.application, &co, &shared_did, "backlog").await, 0);
	assert_eq!(head_task_count(&device_a.application, &co, &shared_did, "doing").await, 0);

	device_a_co
		.push(&owner, CO_CORE_NAME_CO, &CoAction::Tags { action: TagsAction::insert(tags!("revision": "same-did")) })
		.await
		.unwrap();
	let from = shared_did.clone();
	let recipient = shared_did.clone();
	assert_eq!(from, recipient);
	let mut completions = Box::pin(device_a.application.actions().filter_map({
		let co = co.clone();
		let recipient = recipient.clone();
		move |action| {
			let co = co.clone();
			let recipient = recipient.clone();
			async move {
				match action {
					Action::HeadsDeliveryComplete(done) if done.co == co && done.recipient == recipient => Some(done),
					_ => None,
				}
			}
		}
	}));
	push_heads_to_dids(
		device_a.application.context(),
		co,
		from,
		[HeadsRecipient { did: recipient.clone(), connectivity: peer_connectivity(&device_b_network) }],
	)
	.unwrap();

	let admission = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(admission.phase, HeadsDeliveryPhase::Admission);
	assert!(matches!(admission.outcome, HeadsDeliveryOutcome::Queued));
	unblock_preexisting_head_delivery(&device_a.application, &CoId::from("same-did-targeted-heads"), &shared_did).await;
	let done = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(done.phase, HeadsDeliveryPhase::Execution);
	assert!(matches!(done.outcome, HeadsDeliveryOutcome::Delivered { .. }));
	let HeadsDeliveryOutcome::Delivered { peers } = done.outcome else {
		panic!("expected delivery to the second device");
	};
	assert!(peers.contains(&device_b_network.local_peer_id()));
	assert!(!peers.contains(&device_a_network.local_peer_id()));
	wait_for_head_task_count(&device_a.application, &CoId::from("same-did-targeted-heads"), &shared_did, "backlog", 0)
		.await;
	wait_for_head_task_count(&device_a.application, &CoId::from("same-did-targeted-heads"), &shared_did, "doing", 0)
		.await;
}

#[tokio::test]
async fn denied_nonparticipant_is_cancelled_without_send() {
	let guard = MutableGuard::new(GuardMode::Deny);
	let shared = removed_participant_fixture(guard).await;
	let (owner_co, owner) = shared.reducer(0, "targeted-heads").await;
	let (recipient_co, _) = shared.reducer(1, "targeted-heads").await;
	let before = recipient_co.reducer_state().await;
	let recipient = shared.identity(1).identity().to_owned();

	owner_co
		.push(&owner, CO_CORE_NAME_CO, &CoAction::Tags { action: TagsAction::insert(tags!("revision": "denied")) })
		.await
		.unwrap();
	let done = dispatch_and_wait_for_execution(&shared, owner.identity().to_owned(), recipient).await;
	assert_eq!(done.phase, HeadsDeliveryPhase::Execution);
	assert!(matches!(done.outcome, HeadsDeliveryOutcome::Cancelled));
	tokio::time::sleep(Duration::from_millis(250)).await;
	assert_eq!(recipient_co.reducer_state().await, before);
}

struct OfflineTargetFixture {
	owner: helper::instance::Instance,
	#[allow(dead_code)]
	reducer: CoReducer,
	identity: DidKeyIdentity,
	co: CoId,
	recipient: Did,
}

async fn offline_target_fixture(guard: impl AccessGuard + 'static) -> OfflineTargetFixture {
	let mut instances = Instances::new("offline-target");
	let owner = instances.create_builder(move |builder| builder.with_access_guard(guard)).await;
	let identity = owner.create_identity().await;
	let co = CoId::from("offline-target");
	let reducer = owner
		.application
		.create_co(identity.clone(), CreateCo::new(co.clone(), None))
		.await
		.unwrap();
	let recipient = Did::from("did:example:offline-target");
	OfflineTargetFixture { owner, reducer, identity, co, recipient }
}

impl OfflineTargetFixture {
	fn completions(&self) -> std::pin::Pin<Box<dyn Stream<Item = HeadsDeliveryCompleteAction> + Send>> {
		let recipient = self.recipient.clone();
		Box::pin(self.owner.application.actions().filter_map(move |action| {
			let recipient = recipient.clone();
			async move {
				match action {
					Action::HeadsDeliveryComplete(done) if done.recipient == recipient => Some(done),
					_ => None,
				}
			}
		}))
	}

	fn dispatch_delivery(&self) {
		push_heads_to_dids(
			self.owner.application.context(),
			self.co.clone(),
			self.identity.identity().to_owned(),
			[HeadsRecipient { did: self.recipient.clone(), connectivity: CoConnectivity::default() }],
		)
		.unwrap();
	}

	async fn schedule_and_wait_for_phases(&self) -> (HeadsDeliveryCompleteAction, HeadsDeliveryCompleteAction) {
		let mut completions = self.completions();
		self.dispatch_delivery();
		let admission = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
		let execution = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
		assert_eq!(admission.phase, HeadsDeliveryPhase::Admission);
		assert_eq!(execution.phase, HeadsDeliveryPhase::Execution);
		(admission, execution)
	}

	fn execution_completions(&self) -> std::pin::Pin<Box<dyn Stream<Item = HeadsDeliveryCompleteAction> + Send>> {
		let recipient = self.recipient.clone();
		Box::pin(self.owner.application.actions().filter_map(move |action| {
			let recipient = recipient.clone();
			async move {
				match action {
					Action::HeadsDeliveryComplete(done)
						if done.recipient == recipient && done.phase == HeadsDeliveryPhase::Execution =>
					{
						Some(done)
					},
					_ => None,
				}
			}
		}))
	}

	async fn process_and_wait_for_execution(&self) -> HeadsDeliveryCompleteAction {
		let mut executions = self.execution_completions();
		self.owner
			.application
			.handle()
			.dispatch(Action::NetworkQueueProcess { co: Some(self.co.clone()), retry: 0 })
			.unwrap();
		timeout(Duration::from_secs(10), executions.next()).await.unwrap().unwrap()
	}
}

async fn queued_head_task(application: &Application, co: &CoId, recipient: &Did) -> (Task, Block) {
	let local_co = application.context().local_co_reducer().await.unwrap();
	let storage = local_co.storage();
	let task = timeout(Duration::from_secs(10), async {
		loop {
			let state = local_co.reducer_state().await;
			let tasks =
				co_sdk::state::board::tasks(storage.clone(), state, "network_queue".to_owned(), "backlog".to_owned());
			if let Some(task) = tasks
				.try_filter(|task| {
					ready(
						task.tags.string("task-type") == Some("co-heads-did")
							&& task.tags.string("co") == Some(co.as_str())
							&& task.tags.string("recipient") == Some(recipient.as_str()),
					)
				})
				.try_first()
				.await
				.unwrap()
			{
				break task;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("queued head task must return to backlog after automatic processing");
	let block = storage.get(&task.payload.unwrap()).await.unwrap();
	(task, block)
}

async fn head_task_count(application: &Application, co: &CoId, recipient: &Did, list: &str) -> usize {
	let local_co = application.context().local_co_reducer().await.unwrap();
	let tasks = co_sdk::state::board::tasks(
		local_co.storage(),
		local_co.reducer_state().await,
		"network_queue".to_owned(),
		list.to_owned(),
	);
	tasks
		.try_filter(|task| {
			ready(
				task.tags.string("task-type") == Some("co-heads-did")
					&& task.tags.string("co") == Some(co.as_str())
					&& task.tags.string("recipient") == Some(recipient.as_str()),
			)
		})
		.try_collect::<Vec<_>>()
		.await
		.unwrap()
		.len()
}

async fn head_tasks_for_co(application: &Application, co: &CoId, list: &str) -> Vec<Task> {
	let local_co = application.context().local_co_reducer().await.unwrap();
	let tasks = co_sdk::state::board::tasks(
		local_co.storage(),
		local_co.reducer_state().await,
		"network_queue".to_owned(),
		list.to_owned(),
	);
	tasks
		.try_filter(|task| {
			ready(task.tags.string("task-type") == Some("co-heads-did") && task.tags.string("co") == Some(co.as_str()))
		})
		.try_collect()
		.await
		.unwrap()
}

async fn queue_tasks_in_list(application: &Application, list: &str) -> Vec<Task> {
	let local_co = application.context().local_co_reducer().await.unwrap();
	co_sdk::state::board::tasks(
		local_co.storage(),
		local_co.reducer_state().await,
		"network_queue".to_owned(),
		list.to_owned(),
	)
	.try_collect()
	.await
	.unwrap()
}

async fn wait_for_head_task_id(application: &Application, co: &CoId, list: &str, task_id: &str) -> Task {
	timeout(Duration::from_secs(10), async {
		loop {
			if let Some(task) = head_tasks_for_co(application, co, list)
				.await
				.into_iter()
				.find(|task| task.id == task_id)
			{
				break task;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("head-delivery task must appear in the expected queue list")
}

fn dispatch_malformed_head_completion(application: &Application, co: &CoId, task_id: String) {
	application
		.handle()
		.dispatch(Action::NetworkTaskExecute {
			co: co.clone(),
			task_id,
			task_type: "co-heads-did".to_owned(),
			task: Block::new_data(KnownMultiCodec::Raw, b"fixture bootstrap completion".to_vec()),
		})
		.unwrap();
}

/// A state mutation in these online fixtures also schedules an automatic
/// self-delivery. The generic queue is intentionally sequential, so finish a
/// pre-existing claimed generation before waiting for the explicitly admitted
/// recipient. If the explicit task is already the active claim, leave it alone.
async fn unblock_preexisting_head_delivery(application: &Application, co: &CoId, recipient: &Did) {
	let mut completed = BTreeSet::new();
	timeout(Duration::from_secs(10), async {
		loop {
			let backlog = head_tasks_for_co(application, co, "backlog").await;
			let doing = head_tasks_for_co(application, co, "doing").await;
			let target_successor_waiting = backlog
				.iter()
				.any(|task| task.tags.string("recipient") == Some(recipient.as_str()));
			let mut blocking_claim_present = false;
			for task in &doing {
				let is_target = task.tags.string("recipient") == Some(recipient.as_str());
				if !is_target || target_successor_waiting {
					blocking_claim_present = true;
					if completed.insert(task.id.clone()) {
						dispatch_malformed_head_completion(application, co, task.id.clone());
					}
				}
			}
			if blocking_claim_present || doing.is_empty() && !backlog.is_empty() {
				tokio::time::sleep(Duration::from_millis(10)).await;
				continue;
			}
			break;
		}
	})
	.await
	.expect("pre-existing head delivery must stop blocking the explicit task");
}

/// Queue-first startup can leave an automatic self-delivery waiting for the
/// normal 30-second connection timeout. Finish only those already-claimed
/// fixture tasks through the real head-delivery completion path so the online
/// assertions exercise the subsequently admitted task without a long serial
/// queue delay.
async fn drain_bootstrap_head_deliveries(application: &Application, co: &CoId) {
	let mut completed = BTreeSet::new();
	timeout(Duration::from_secs(10), async {
		loop {
			for task in head_tasks_for_co(application, co, "doing").await {
				if completed.insert(task.id.clone()) {
					dispatch_malformed_head_completion(application, co, task.id);
				}
			}
			if head_tasks_for_co(application, co, "backlog").await.is_empty()
				&& head_tasks_for_co(application, co, "doing").await.is_empty()
			{
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("bootstrap head-delivery tasks must settle");
}

async fn wait_for_head_task_count(application: &Application, co: &CoId, recipient: &Did, list: &str, expected: usize) {
	timeout(Duration::from_secs(10), async {
		loop {
			if head_task_count(application, co, recipient, list).await == expected {
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.unwrap();
}

#[tokio::test]
async fn offline_recipient_creates_one_backlog_task() {
	let fixture = offline_target_fixture(MutableGuard::new(GuardMode::Allow)).await;
	let (admission, execution) = fixture.schedule_and_wait_for_phases().await;
	assert_eq!(admission.attempted_heads, None);
	assert!(matches!(admission.outcome, HeadsDeliveryOutcome::Queued));
	assert!(execution.attempted_heads.is_some());
	assert!(matches!(execution.outcome, HeadsDeliveryOutcome::Queued));
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "backlog", 1).await;
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "failed", 0).await;
}

#[tokio::test]
async fn successful_admission_triggers_processing_after_completion() {
	let fixture = offline_target_fixture(MutableGuard::new(GuardMode::Allow)).await;
	let co = fixture.co.clone();
	let recipient = fixture.recipient.clone();
	let mut actions = Box::pin(fixture.owner.application.actions().filter(move |action| {
		ready(match action {
			Action::HeadsDeliveryComplete(done) => {
				done.co == co
					&& done.recipient == recipient
					&& done.phase == HeadsDeliveryPhase::Admission
					&& matches!(done.outcome, HeadsDeliveryOutcome::Queued)
			},
			Action::NetworkQueueProcess { co: Some(process_co), retry: 0 } => process_co == &co,
			_ => false,
		})
	}));

	push_heads_to_dids(
		fixture.owner.application.context(),
		fixture.co.clone(),
		fixture.identity.identity().to_owned(),
		[HeadsRecipient { did: fixture.recipient.clone(), connectivity: CoConnectivity::default() }],
	)
	.unwrap();

	assert!(matches!(
		timeout(Duration::from_secs(10), actions.next()).await.unwrap().unwrap(),
		Action::HeadsDeliveryComplete(_)
	));
	assert!(matches!(
		timeout(Duration::from_secs(1), actions.next())
			.await
			.expect("successful admission must trigger queue processing")
			.unwrap(),
		Action::NetworkQueueProcess { co: Some(process_co), retry: 0 } if process_co == fixture.co
	));
}

#[tokio::test]
async fn unclaimed_network_task_execute_is_failed_before_authorization() {
	let guard = CountingGuard::new(GuardMode::Allow);
	let fixture = offline_target_fixture(guard.clone()).await;
	fixture.schedule_and_wait_for_phases().await;
	let (queued, block) = queued_head_task(&fixture.owner.application, &fixture.co, &fixture.recipient).await;
	let local_co = fixture.owner.application.context().local_co_reducer().await.unwrap();
	let local_identity = fixture.owner.application.context().local_identity();
	local_co
		.push(&local_identity, network_queue_core(), &BoardAction::TaskDelete(queued.id))
		.await
		.unwrap();
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "backlog", 0).await;

	guard.set(GuardMode::Deny);
	guard.reset();
	let task_id = "unclaimed-head-delivery".to_owned();
	let mut completions = Box::pin(fixture.owner.application.actions().filter_map({
		let task_id = task_id.clone();
		move |action| {
			let task_id = task_id.clone();
			async move {
				match action {
					Action::NetworkTaskExecuteComplete { task_id: completed, task_state, .. }
						if completed == task_id =>
					{
						Some(task_state)
					},
					_ => None,
				}
			}
		}
	}));
	fixture
		.owner
		.application
		.handle()
		.dispatch(Action::NetworkTaskExecute {
			co: fixture.co.clone(),
			task_id,
			task_type: "co-heads-did".to_owned(),
			task: block,
		})
		.unwrap();

	let state = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(state.list_name(), Some("failed"));
	assert_eq!(guard.calls(), 0, "an unclaimed task must not reach authorization");
}

#[tokio::test]
async fn claimed_task_with_recipient_tag_payload_mismatch_is_failed_before_authorization() {
	let guard = CountingGuard::new(GuardMode::Allow);
	let fixture = offline_target_fixture(guard.clone()).await;
	fixture.schedule_and_wait_for_phases().await;
	let (queued, _) = queued_head_task(&fixture.owner.application, &fixture.co, &fixture.recipient).await;
	let local_co = fixture.owner.application.context().local_co_reducer().await.unwrap();
	let local_identity = fixture.owner.application.context().local_identity();
	local_co
		.push(&local_identity, network_queue_core(), &BoardAction::TaskDelete(queued.id))
		.await
		.unwrap();
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "backlog", 0).await;

	let payload_recipient = Did::from("did:example:payload-recipient");
	let block = BlockSerializer::default()
		.serialize(&SerializedHeadsDeliveryIntent {
			co: fixture.co.clone(),
			from: fixture.identity.identity().to_owned(),
			recipient: payload_recipient,
			connectivity: CoConnectivity::default(),
		})
		.unwrap();
	let payload = local_co.storage().set(block.clone()).await.unwrap();
	let task_id = "mismatched-claimed-head-delivery".to_owned();
	local_co
		.push(
			&local_identity,
			network_queue_core(),
			&BoardAction::TaskCreate {
				list: "doing".to_owned(),
				task: Task {
					id: task_id.clone(),
					name: "mismatched claimed head delivery".to_owned(),
					tags: tags!(
						"co": fixture.co.to_string(),
						"task-type": "co-heads-did",
						"recipient": fixture.recipient.clone(),
					),
					payload: Some(payload),
					lock: Some("validation-test-claim".to_owned()),
				},
				after: None,
			},
		)
		.await
		.unwrap();

	guard.set(GuardMode::Deny);
	guard.reset();
	let mut completions = Box::pin(fixture.owner.application.actions().filter_map({
		let task_id = task_id.clone();
		move |action| {
			let task_id = task_id.clone();
			async move {
				match action {
					Action::NetworkTaskExecuteComplete { task_id: completed, task_state, .. }
						if completed == task_id =>
					{
						Some(task_state)
					},
					_ => None,
				}
			}
		}
	}));
	fixture
		.owner
		.application
		.handle()
		.dispatch(Action::NetworkTaskExecute {
			co: fixture.co.clone(),
			task_id,
			task_type: "co-heads-did".to_owned(),
			task: block,
		})
		.unwrap();

	let state = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(state.list_name(), Some("failed"));
	assert_eq!(guard.calls(), 0, "mismatched routing tags must not reach authorization");
}

#[tokio::test]
async fn stale_denial_settles_claim_without_deleting_the_newer_backlog() {
	let guard = StaleDenyGuard::default();
	let fixture = offline_target_fixture(guard.clone()).await;
	let mut completions = fixture.completions();

	fixture.dispatch_delivery();
	let first_admission = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(first_admission.phase, HeadsDeliveryPhase::Admission);
	assert!(matches!(first_admission.outcome, HeadsDeliveryOutcome::Queued));
	timeout(Duration::from_secs(10), guard.first_check_started.notified())
		.await
		.expect("first queued execution must start authorization");

	fixture.dispatch_delivery();
	let successor = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(successor.phase, HeadsDeliveryPhase::Admission);
	assert!(matches!(successor.outcome, HeadsDeliveryOutcome::Queued));
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "backlog", 1).await;

	guard.release_first_check.notify_one();
	let settled = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(settled.phase, HeadsDeliveryPhase::Execution);
	assert!(matches!(settled.outcome, HeadsDeliveryOutcome::Cancelled));
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "backlog", 1).await;
}

#[tokio::test]
async fn revocation_after_admission_cancels_execution_and_deletes_task() {
	let guard = GatedGuard::new(GuardMode::Allow);
	let fixture = offline_target_fixture(guard.clone()).await;
	let mut completions = fixture.completions();
	fixture.dispatch_delivery();
	let queued = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(queued.phase, HeadsDeliveryPhase::Admission);
	assert!(matches!(queued.outcome, HeadsDeliveryOutcome::Queued));
	timeout(Duration::from_secs(10), guard.first_check_started.notified())
		.await
		.expect("execution must read authorization after admission");
	guard.set(GuardMode::Deny);
	guard.release();
	let done = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(done.phase, HeadsDeliveryPhase::Execution);
	assert!(matches!(done.outcome, HeadsDeliveryOutcome::Cancelled));
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "backlog", 0).await;
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "doing", 0).await;
}

#[tokio::test]
async fn execution_uses_current_heads_after_admission() {
	let guard = GatedGuard::new(GuardMode::Allow);
	let fixture = offline_target_fixture(guard.clone()).await;
	let mut completions = fixture.completions();
	fixture.dispatch_delivery();
	let admission = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(admission.phase, HeadsDeliveryPhase::Admission);
	assert_eq!(admission.attempted_heads, None);
	timeout(Duration::from_secs(10), guard.first_check_started.notified())
		.await
		.expect("execution must wait in current authorization");

	fixture
		.reducer
		.push(
			&fixture.identity,
			CO_CORE_NAME_CO,
			&CoAction::Tags { action: TagsAction::insert(tags!("newer": "state")) },
		)
		.await
		.unwrap();
	let current = fixture.reducer.reducer_state().await.heads();
	guard.release();

	let done = timeout(Duration::from_secs(10), completions.next()).await.unwrap().unwrap();
	assert_eq!(done.phase, HeadsDeliveryPhase::Execution);
	assert_eq!(done.attempted_heads.as_ref(), Some(&current));
	assert!(matches!(done.outcome, HeadsDeliveryOutcome::Queued));
}

#[tokio::test]
async fn guard_error_keeps_execution_in_backlog() {
	let guard = MutableGuard::new(GuardMode::Allow);
	let fixture = offline_target_fixture(guard.clone()).await;
	fixture.schedule_and_wait_for_phases().await;
	guard.set(GuardMode::Error);
	let done = fixture.process_and_wait_for_execution().await;
	assert_eq!(done.phase, HeadsDeliveryPhase::Execution);
	assert!(matches!(done.outcome, HeadsDeliveryOutcome::Queued));
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "backlog", 1).await;
	wait_for_head_task_count(&fixture.owner.application, &fixture.co, &fixture.recipient, "failed", 0).await;
}

#[tokio::test]
async fn network_start_triggers_persisted_head_execution() {
	let guard = MutableGuard::new(GuardMode::Allow);
	let mut fixture = offline_target_fixture(guard).await;
	let task_id = "persisted-head-delivery-before-network-start".to_owned();
	let block = BlockSerializer::default()
		.serialize(&SerializedHeadsDeliveryIntent {
			co: fixture.co.clone(),
			from: fixture.identity.identity().to_owned(),
			recipient: fixture.recipient.clone(),
			connectivity: CoConnectivity::default(),
		})
		.unwrap();
	fixture
		.owner
		.application
		.handle()
		.dispatch(Action::NetworkTaskQueue {
			co: fixture.co.clone(),
			task_id: task_id.clone(),
			task_type: "co-heads-did".to_owned(),
			task_name: "persisted head delivery before network start".to_owned(),
			task: block,
		})
		.unwrap();
	wait_for_head_task_id(&fixture.owner.application, &fixture.co, "backlog", &task_id).await;
	let local_co = fixture.owner.application.context().local_co_reducer().await.unwrap();
	let local_identity = fixture.owner.application.context().local_identity();
	local_co
		.push(
			&local_identity,
			network_queue_core(),
			&BoardAction::TaskTags(task_id.clone(), TagsAction::insert(tags!("recipient": fixture.recipient.clone()))),
		)
		.await
		.unwrap();

	let mut startup = Box::pin(fixture.owner.application.actions().filter({
		let task_id = task_id.clone();
		move |action| {
			ready(match action {
				Action::NetworkStartComplete(Ok(())) => true,
				Action::NetworkQueueProcess { co: None, retry: 0 } => true,
				Action::NetworkTaskExecute { task_id: executing, .. } => executing == &task_id,
				_ => false,
			})
		}
	}));
	fixture
		.owner
		.application
		.create_network(co_sdk::NetworkSettings::default().with_localhost())
		.await
		.unwrap();
	assert!(matches!(
		timeout(Duration::from_secs(10), startup.next()).await.unwrap().unwrap(),
		Action::NetworkStartComplete(Ok(()))
	));
	assert!(matches!(
		timeout(Duration::from_secs(1), startup.next()).await.unwrap().unwrap(),
		Action::NetworkQueueProcess { co: None, retry: 0 }
	));
	assert!(matches!(
		timeout(Duration::from_secs(10), startup.next()).await.unwrap().unwrap(),
		Action::NetworkTaskExecute { task_id: executing, .. } if executing == task_id
	));
}

fn concurrent_head_selector(co: &CoId, recipient: &Did) -> TagsExpr {
	TagsExpr::new("task-type", "co-heads-did")
		.and(TagsExpr::new("co", co.to_string()))
		.and(TagsExpr::new("recipient", recipient.to_string()))
}

fn matches_concurrent_head(task: &Task, co: &CoId, recipient: &Did) -> bool {
	task.tags.string("task-type") == Some("co-heads-did")
		&& task.tags.string("co") == Some(co.as_str())
		&& task.tags.string("recipient") == Some(recipient.as_str())
}

async fn assert_concurrent_local_instances_order(case: &str, completion_first: bool) {
	let tmp = test_tmp_dir();
	let application_a = ApplicationBuilder::new_with_path(
		format!("{}-concurrent-local-instances-{case}-a", tmp.uuid()),
		tmp.path().to_owned(),
	)
	.without_keychain()
	.with_disabled_feature("co-local-watch")
	.with_disabled_feature("co-local-encryption")
	.with_co_uuid(MonotonicCoUuid::default())
	.build()
	.await
	.expect("application A");
	let local_a = application_a.local_co_reducer().await.expect("local CO A");

	// Initialize (or upgrade) the real persisted queue before the second Local CO opens.
	let bootstrap_co = CoId::from(format!("concurrent-local-instances-{case}-bootstrap"));
	let bootstrap_id = format!("concurrent-local-instances-{case}-bootstrap-task");
	application_a
		.handle()
		.dispatch(Action::NetworkTaskQueue {
			co: bootstrap_co.clone(),
			task_id: bootstrap_id.clone(),
			task_type: "co-heads-did".to_owned(),
			task_name: "initialize concurrent Local CO queue".to_owned(),
			task: Block::new_data(KnownMultiCodec::Raw, case.as_bytes().to_vec()),
		})
		.unwrap();
	wait_for_head_task_id(&application_a, &bootstrap_co, "backlog", &bootstrap_id).await;
	local_a
		.push(&application_a.local_identity(), network_queue_core(), &BoardAction::TaskDelete(bootstrap_id))
		.await
		.unwrap();

	let co = CoId::from(format!("concurrent-local-instances-{case}"));
	let recipient = Did::from(format!("did:example:{case}-target"));
	let other_recipient = Did::from(format!("did:example:{case}-other"));
	let stale_id = format!("concurrent-local-instances-{case}-stale");
	let fresh_id = format!("concurrent-local-instances-{case}-fresh");
	let lock = format!("concurrent-local-instances-{case}-claim");
	let blocker_payload_a = local_a
		.storage()
		.set(Block::new_data(KnownMultiCodec::Raw, format!("{case}-blocker-a").into_bytes()))
		.await
		.unwrap();
	let blocker_payload_b = local_a
		.storage()
		.set(Block::new_data(KnownMultiCodec::Raw, format!("{case}-blocker-b").into_bytes()))
		.await
		.unwrap();
	let blocker_a = Task {
		id: format!("concurrent-local-instances-{case}-blocker-a"),
		name: "hold application A queue processor".to_owned(),
		tags: tags!(
			"co": format!("concurrent-local-instances-{case}-blocker"),
			"task-type": "convergence-blocker",
		),
		payload: Some(blocker_payload_a),
		lock: None,
	};
	let blocker_b = Task {
		id: format!("concurrent-local-instances-{case}-blocker-b"),
		name: "hold application B queue processor".to_owned(),
		tags: tags!(
			"co": format!("concurrent-local-instances-{case}-blocker"),
			"task-type": "convergence-blocker",
		),
		payload: Some(blocker_payload_b),
		lock: None,
	};
	let stale = Task {
		id: stale_id.clone(),
		name: "stale target generation".to_owned(),
		tags: tags!(
			"co": co.to_string(),
			"task-type": "co-heads-did",
			"recipient": recipient.clone(),
			"generation": "stale",
		),
		payload: None,
		lock: None,
	};
	let fresh = Task {
		id: fresh_id.clone(),
		name: "fresh target generation".to_owned(),
		tags: tags!(
			"co": co.to_string(),
			"task-type": "co-heads-did",
			"recipient": recipient.clone(),
			"generation": "fresh",
		),
		payload: None,
		lock: None,
	};
	let unrelated_head = Task {
		id: format!("concurrent-local-instances-{case}-other-head"),
		name: "unrelated recipient head generation".to_owned(),
		tags: tags!(
			"co": co.to_string(),
			"task-type": "co-heads-did",
			"recipient": other_recipient,
		),
		payload: None,
		lock: None,
	};
	let unrelated_didcomm = Task {
		id: format!("concurrent-local-instances-{case}-didcomm"),
		name: "unrelated DIDComm task".to_owned(),
		tags: tags!(
			"co": co.to_string(),
			"task-type": "did-didcomm",
			"recipient": recipient.clone(),
		),
		payload: None,
		lock: None,
	};
	for task in [&blocker_a, &blocker_b, &unrelated_head, &unrelated_didcomm] {
		local_a
			.push(
				&application_a.local_identity(),
				network_queue_core(),
				&BoardAction::TaskCreate { list: "backlog".to_owned(), task: task.clone(), after: None },
			)
			.await
			.unwrap();
	}

	let mut blocker_a_executions = Box::pin(application_a.actions().filter({
		let blocker_a_id = blocker_a.id.clone();
		move |action| {
			ready(matches!(
				action,
				Action::NetworkTaskExecute { task_id, task_type, .. }
					if task_id == &blocker_a_id && task_type == "convergence-blocker"
			))
		}
	}));
	application_a
		.handle()
		.dispatch(Action::NetworkQueueProcess { co: None, retry: 0 })
		.unwrap();
	assert!(matches!(
		timeout(Duration::from_secs(5), blocker_a_executions.next()).await.unwrap().unwrap(),
		Action::NetworkTaskExecute { task_id, .. } if task_id == blocker_a.id
	));
	drop(blocker_a_executions);

	let application_b = ApplicationBuilder::new_with_path(
		format!("{}-concurrent-local-instances-{case}-b", tmp.uuid()),
		tmp.path().to_owned(),
	)
	.without_keychain()
	.with_disabled_feature("co-local-watch")
	.with_disabled_feature("co-local-encryption")
	.with_co_uuid(MonotonicCoUuid::default())
	.build()
	.await
	.expect("application B");
	let local_b = application_b.local_co_reducer().await.expect("local CO B");
	let mut blocker_b_executions = Box::pin(application_b.actions().filter({
		let blocker_b_id = blocker_b.id.clone();
		move |action| {
			ready(matches!(
				action,
				Action::NetworkTaskExecute { task_id, task_type, .. }
					if task_id == &blocker_b_id && task_type == "convergence-blocker"
			))
		}
	}));
	application_b
		.handle()
		.dispatch(Action::NetworkQueueProcess { co: None, retry: 0 })
		.unwrap();
	assert!(matches!(
		timeout(Duration::from_secs(5), blocker_b_executions.next()).await.unwrap().unwrap(),
		Action::NetworkTaskExecute { task_id, .. } if task_id == blocker_b.id
	));
	drop(blocker_b_executions);
	timeout(Duration::from_secs(5), async {
		loop {
			application_a.co().refresh(local_a.clone()).await.unwrap();
			application_b.co().refresh(local_b.clone()).await.unwrap();
			if local_a.reducer_state().await == local_b.reducer_state().await {
				break;
			}
			tokio::task::yield_now().await;
		}
	})
	.await
	.expect("Local CO reducers must share an initialized queue state");

	let actor_a = DidKeyIdentity::generate(None);
	let actor_b = DidKeyIdentity::generate(None);
	let (lower_actor, higher_actor) = if actor_a.identity().as_bytes() < actor_b.identity().as_bytes() {
		(&actor_a, &actor_b)
	} else {
		(&actor_b, &actor_a)
	};
	let (completion_actor, enqueue_actor) =
		if completion_first { (lower_actor, higher_actor) } else { (higher_actor, lower_actor) };
	// Both branches start from the same state and contain three pushes. co-log
	// therefore breaks the H+3 completion/enqueue tie by actor clock ID.
	assert_eq!(
		completion_actor.identity().as_bytes() < enqueue_actor.identity().as_bytes(),
		completion_first,
		"the actor clock IDs force the requested H+3 action order"
	);

	let stale_completion_branch = async {
		local_a
			.push(
				completion_actor,
				network_queue_core(),
				&BoardAction::TaskEnqueueOrReplace {
					list: "backlog".to_owned(),
					matching: concurrent_head_selector(&co, &recipient),
					task: stale.clone(),
					after: None,
				},
			)
			.await
			.unwrap();
		local_a
			.push(
				completion_actor,
				network_queue_core(),
				&BoardAction::TaskMove {
					from_list: Some("backlog".to_owned()),
					list: "doing".to_owned(),
					task: stale_id.clone(),
					after: None,
					lock: TaskLock::Lock(lock.clone()),
				},
			)
			.await
			.unwrap();
		local_a
			.push(
				completion_actor,
				network_queue_core(),
				&BoardAction::TaskCompleteIf {
					task: stale_id.clone(),
					from_list: "doing".to_owned(),
					expected_lock: lock.clone(),
					transition: TaskTransition::Move {
						list: "backlog".to_owned(),
						after: None,
						unless_matching: Some(concurrent_head_selector(&co, &recipient)),
					},
				},
			)
			.await
			.unwrap()
	};
	let fresh_enqueue_branch = async {
		// Missing-task completions are successful reducer no-ops. They align the
		// fresh enqueue with the other branch's H+3 completion.
		for sequence in [1, 2] {
			local_b
				.push(
					enqueue_actor,
					network_queue_core(),
					&BoardAction::TaskCompleteIf {
						task: format!("concurrent-local-instances-{case}-missing-{sequence}"),
						from_list: "doing".to_owned(),
						expected_lock: format!("concurrent-local-instances-{case}-missing-lock"),
						transition: TaskTransition::Delete,
					},
				)
				.await
				.unwrap();
		}
		local_b
			.push(
				enqueue_actor,
				network_queue_core(),
				&BoardAction::TaskEnqueueOrReplace {
					list: "backlog".to_owned(),
					matching: concurrent_head_selector(&co, &recipient),
					task: fresh.clone(),
					after: None,
				},
			)
			.await
			.unwrap()
	};
	let (completion_state, enqueue_state) = if completion_first {
		let completion_state = stale_completion_branch.await;
		let enqueue_state = fresh_enqueue_branch.await;
		(completion_state, enqueue_state)
	} else {
		let enqueue_state = fresh_enqueue_branch.await;
		let completion_state = stale_completion_branch.await;
		(completion_state, enqueue_state)
	};
	let completion_heads = completion_state.heads();
	let enqueue_heads = enqueue_state.heads();
	assert_eq!(completion_heads.len(), 1, "terminal completion has one head");
	assert_eq!(enqueue_heads.len(), 1, "terminal enqueue has one head");
	let completion_entry = EntryBlock::from_block(
		local_a
			.storage()
			.get(completion_heads.first().expect("completion head"))
			.await
			.unwrap(),
	)
	.unwrap();
	let enqueue_entry = EntryBlock::from_block(
		local_b
			.storage()
			.get(enqueue_heads.first().expect("enqueue head"))
			.await
			.unwrap(),
	)
	.unwrap();
	assert_eq!(completion_entry.entry().clock.time, enqueue_entry.entry().clock.time);
	assert_eq!(completion_entry.entry().clock.id.as_slice(), completion_actor.identity().as_bytes());
	assert_eq!(enqueue_entry.entry().clock.id.as_slice(), enqueue_actor.identity().as_bytes());
	assert_eq!(completion_entry.entry().clock.id < enqueue_entry.entry().clock.id, completion_first);
	assert_ne!(local_a.reducer_state().await, local_b.reducer_state().await);

	// Each live reducer initially contains only its local branch. Explicitly
	// refresh both applications while their queue processors remain parked.
	timeout(Duration::from_secs(5), async {
		loop {
			if completion_first {
				application_a.co().refresh(local_a.clone()).await.unwrap();
				application_b.co().refresh(local_b.clone()).await.unwrap();
			} else {
				application_b.co().refresh(local_b.clone()).await.unwrap();
				application_a.co().refresh(local_a.clone()).await.unwrap();
			}
			if local_a.reducer_state().await == local_b.reducer_state().await {
				break;
			}
			tokio::task::yield_now().await;
		}
	})
	.await
	.expect("concurrent Local CO queue reducers must converge");
	assert_eq!(local_a.reducer_state().await, local_b.reducer_state().await);

	let backlog = queue_tasks_in_list(&application_a, "backlog").await;
	let matching: Vec<_> = backlog
		.iter()
		.filter(|task| matches_concurrent_head(task, &co, &recipient))
		.cloned()
		.collect();
	assert!(matching.len() <= 1, "at most one target generation may remain");
	assert!(matching.iter().all(|task| task.lock.is_none()));
	assert_eq!(matching, [fresh]);
	for list in ["backlog", "doing", "failed", "done"] {
		assert!(
			queue_tasks_in_list(&application_a, list)
				.await
				.iter()
				.all(|task| task.id != stale_id),
			"the stale generation must be removed from {list}"
		);
	}
	assert_eq!(backlog.iter().find(|task| task.id == unrelated_head.id), Some(&unrelated_head));
	assert_eq!(backlog.iter().find(|task| task.id == unrelated_didcomm.id), Some(&unrelated_didcomm));
	let doing = queue_tasks_in_list(&application_a, "doing").await;
	for expected in [&blocker_a, &blocker_b] {
		let actual = doing
			.iter()
			.find(|task| task.id == expected.id)
			.expect("queue blocker remains claimed");
		let mut expected_claimed = expected.clone();
		expected_claimed.lock = actual.lock.clone();
		assert_eq!(actual, &expected_claimed);
		assert!(actual.lock.is_some());
	}

	application_b.shutdown_application().await;
	application_a.shutdown_application().await;
}

#[tokio::test]
async fn concurrent_local_instances_converge_for_completion_and_enqueue_orders() {
	co_test::init_test_log();
	assert_concurrent_local_instances_order("completion-then-enqueue", true).await;
	assert_concurrent_local_instances_order("enqueue-then-completion", false).await;
}

#[tokio::test]
async fn joined_queue_mutation_wakes_idle_network_queue_processor() {
	co_test::init_test_log();
	let tmp = test_tmp_dir();
	let application_a =
		ApplicationBuilder::new_with_path(format!("{}-joined-queue-a", tmp.uuid()), tmp.path().to_owned())
			.without_keychain()
			.with_disabled_feature("co-local-encryption")
			.build()
			.await
			.expect("application A");
	let local_a = application_a.local_co_reducer().await.expect("local CO A");

	// Initialize the queue core through the public action path before B joins.
	let bootstrap_co = CoId::from("joined-queue-bootstrap");
	let bootstrap_task = "joined-queue-bootstrap-task".to_owned();
	application_a
		.handle()
		.dispatch(Action::NetworkTaskQueue {
			co: bootstrap_co.clone(),
			task_id: bootstrap_task.clone(),
			task_type: "co-heads-did".to_owned(),
			task_name: "initialize joined queue fixture".to_owned(),
			task: Block::new_data(KnownMultiCodec::Raw, b"joined queue bootstrap".to_vec()),
		})
		.unwrap();
	wait_for_head_task_id(&application_a, &bootstrap_co, "backlog", &bootstrap_task).await;
	local_a
		.push(&application_a.local_identity(), network_queue_core(), &BoardAction::TaskDelete(bootstrap_task))
		.await
		.unwrap();
	timeout(Duration::from_secs(3), async {
		loop {
			if head_tasks_for_co(&application_a, &bootstrap_co, "backlog").await.is_empty() {
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("bootstrap task must be deleted");

	// Open B only after the initialized queue is empty. Its initial scan must
	// therefore become idle before the observed mutation below.
	let application_b =
		ApplicationBuilder::new_with_path(format!("{}-joined-queue-b", tmp.uuid()), tmp.path().to_owned())
			.without_keychain()
			.with_disabled_feature("co-local-encryption")
			.build()
			.await
			.expect("application B");
	let local_b = application_b.local_co_reducer().await.expect("local CO B");
	timeout(Duration::from_secs(3), async {
		loop {
			if local_a.reducer_state().await == local_b.reducer_state().await {
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("B must open at A's initialized, empty queue state");

	let mut startup_completions = Box::pin(application_b.actions().filter(|action| {
		ready(matches!(action, Action::NetworkQueueProcessComplete { co: None, is_empty: true, retry: 0 }))
	}));
	application_b
		.handle()
		.dispatch(Action::NetworkQueueProcess { co: None, retry: 0 })
		.unwrap();
	assert!(matches!(
		timeout(Duration::from_secs(3), startup_completions.next())
			.await
			.expect("the initial empty queue scan must complete")
			.unwrap(),
		Action::NetworkQueueProcessComplete { co: None, is_empty: true, retry: 0 }
	));
	drop(startup_completions);

	let mut joined_queue_actions = Box::pin(application_b.actions().filter(|action| {
		ready(matches!(
			action,
			Action::CoreAction { co, context, action, .. }
				if co.as_str() == CO_ID_LOCAL
					&& !context.is_local_change()
					&& !context.is_initialize()
					&& CoreName::<Board>::new("network_queue") == action.core
		))
	}));
	let mut joined_queue_processes = Box::pin(
		application_b
			.actions()
			.filter(|action| ready(matches!(action, Action::NetworkQueueProcess { co: None, retry: 0 }))),
	);

	local_a
		.push(
			&application_a.local_identity(),
			network_queue_core(),
			&BoardAction::TaskCreate {
				list: "backlog".to_owned(),
				task: Task {
					id: "joined-queue-observed-task".to_owned(),
					name: "wake joined queue processor".to_owned(),
					tags: tags!("co": "joined-queue-observed", "task-type": "joined-queue-regression"),
					payload: None,
					lock: None,
				},
				after: None,
			},
		)
		.await
		.unwrap();

	timeout(Duration::from_secs(3), async {
		loop {
			if local_a.reducer_state().await == local_b.reducer_state().await {
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("local reducers must converge after app A enqueues the task");
	let joined_action = timeout(Duration::from_secs(1), joined_queue_actions.next()).await;
	let joined_process = timeout(Duration::from_secs(1), joined_queue_processes.next()).await;
	drop(joined_queue_actions);
	drop(joined_queue_processes);
	application_b.shutdown_application().await;
	application_a.shutdown_application().await;
	assert!(matches!(joined_action, Ok(Some(Action::CoreAction { .. }))));
	assert!(
		matches!(joined_process, Ok(Some(Action::NetworkQueueProcess { co: None, retry: 0 }))),
		"the joined queue mutation must wake app B's idle processor"
	);
}
