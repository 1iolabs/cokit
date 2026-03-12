// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::library::locals::{ApplicationLocal, Locals};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::{ActorError, ActorHandle, JsLocalTaskSpawner, LocalActor, Response, ResponseStream, ResponseStreams};
use co_primitives::{from_cbor, to_cbor};
use futures::{Stream, StreamExt};
use std::mem::take;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
	BroadcastChannel, IdbDatabase, IdbFactory, IdbObjectStore, IdbOpenDbRequest, IdbRequest, IdbTransaction,
	IdbTransactionMode,
};

const OBJECT_STORE_NAME: &str = "locals";
const DB_VERSION: u32 = 1;
const LOCAL_KEY: &str = "local";

/// IndexedDB-backed persistent locals for WASM.
///
/// Uses the [`LocalActor`] pattern to keep `!Send` IDB handles off the caller's thread.
/// The database is opened during actor initialization (inside `spawn_local`).
///
/// Cross-tab notifications are delivered via [`BroadcastChannel`]: when one tab writes,
/// every other tab re-reads from IDB and pushes the update to active watchers.
#[derive(Debug, Clone)]
pub struct IndexedDbLocals {
	handle: ActorHandle<IdbLocalsMessage>,
}
impl IndexedDbLocals {
	/// Create a new IndexedDB-backed locals handle.
	///
	/// # Args
	/// - `db_name` is the IndexedDB database name, e.g. `"co-locals::my-app"`
	pub fn new(db_name: impl Into<String>) -> Result<Self, anyhow::Error> {
		let db_name: String = db_name.into();

		let instance =
			LocalActor::spawn_with(JsLocalTaskSpawner::default(), Default::default(), IdbLocalsActor, db_name)
				.map_err(|e| anyhow!("actor spawn failed: {:?}", e))?;

		Ok(Self { handle: instance.handle() })
	}
}
#[async_trait]
impl Locals for IndexedDbLocals {
	#[tracing::instrument(level = tracing::Level::TRACE, name = "indexeddb-locals-get", err(Debug), ret)]
	async fn get(&self) -> Result<Vec<ApplicationLocal>, anyhow::Error> {
		self.handle
			.request(|response| IdbLocalsMessage::Get(response))
			.await
			.map_err(|e| anyhow!("IndexedDB actor error: {:?}", e))?
	}

	#[tracing::instrument(level = tracing::Level::TRACE, name = "indexeddb-locals-set", err(Debug))]
	async fn set(&mut self, local: ApplicationLocal) -> Result<(), anyhow::Error> {
		self.handle
			.request(|response| IdbLocalsMessage::Set(local, response))
			.await
			.map_err(|e| anyhow!("IndexedDB actor error: {:?}", e))?
	}

	fn watch(&self) -> impl Stream<Item = ApplicationLocal> + Send + Sync + 'static {
		self.handle
			.stream(IdbLocalsMessage::Watch)
			.filter_map(|item: Result<ApplicationLocal, ActorError>| std::future::ready(item.ok()))
	}
}

#[derive(Debug)]
enum IdbLocalsMessage {
	Get(Response<Result<Vec<ApplicationLocal>, anyhow::Error>>),
	Set(ApplicationLocal, Response<Result<(), anyhow::Error>>),
	Watch(ResponseStream<ApplicationLocal>),
	/// Notification from another tab via BroadcastChannel.
	Notify,
}

struct IdbLocalsState {
	db_name: String,
	db: IdbDatabase,
	channel: Option<BroadcastChannel>,
	watchers: ResponseStreams<ApplicationLocal>,
}

#[derive(Debug)]
struct IdbLocalsActor;
impl IdbLocalsActor {
	fn store(db: &IdbDatabase, mode: IdbTransactionMode) -> Result<IdbObjectStore, anyhow::Error> {
		let tx: IdbTransaction = db
			.transaction_with_str_and_mode(OBJECT_STORE_NAME, mode)
			.map_err(|e| anyhow!("IDB transaction failed: {:?}", e))?;
		tx.object_store(OBJECT_STORE_NAME)
			.map_err(|e| anyhow!("IDB object_store failed: {:?}", e))
	}

	async fn handle_get(db: &IdbDatabase) -> Result<Vec<ApplicationLocal>, anyhow::Error> {
		let store = Self::store(db, IdbTransactionMode::Readonly)?;
		let key = JsValue::from_str(LOCAL_KEY);
		let request: IdbRequest = store.get(&key).map_err(|e| anyhow!("IDB get failed: {:?}", e))?;
		let result = idb_request_await(&request).await?;

		if result.is_undefined() || result.is_null() {
			return Ok(Vec::new());
		}

		let bytes = bytes_from_js(&result).map_err(|e| anyhow!("IDB decode failed: {:?}", e))?;
		let local: ApplicationLocal = from_cbor(&bytes)?;
		Ok(vec![local])
	}

	async fn handle_set(state: &IdbLocalsState, local: ApplicationLocal) -> Result<(), anyhow::Error> {
		let data = to_cbor(&local)?;

		let store = Self::store(&state.db, IdbTransactionMode::Readwrite)?;
		let key = JsValue::from_str(LOCAL_KEY);
		let value = bytes_to_js(&data);
		let request: IdbRequest = store
			.put_with_key(&value, &key)
			.map_err(|e| anyhow!("IDB put failed: {:?}", e))?;
		idb_request_await(&request).await?;

		// notify other tabs.
		if let Some(channel) = &state.channel {
			channel
				.post_message(&JsValue::NULL)
				.map_err(|e| anyhow!("BroadcastChannel post failed: {:?}", e))?;
		}

		Ok(())
	}

	async fn handle_notify(state: &mut IdbLocalsState) {
		if state.watchers.is_closed() {
			return;
		}
		match Self::handle_get(&state.db).await {
			Ok(locals) => {
				for local in locals {
					state.watchers.send(local);
				}
			},
			Err(err) => tracing::warn!(?err, "indexeddb-locals-notify-read-failed"),
		}
		Self::maybe_stop_watch(state);
	}

	/// Listen for cross-tab notifications.
	fn start_watch(handle: ActorHandle<IdbLocalsMessage>, db_name: &str) -> Result<BroadcastChannel, anyhow::Error> {
		let channel = BroadcastChannel::new(db_name)
			.map_err(|e| ActorError::Actor(anyhow!("BroadcastChannel open failed: {:?}", e)))?;
		let on_message = Closure::wrap(Box::new(move |_: web_sys::MessageEvent| {
			handle.dispatch(IdbLocalsMessage::Notify).ok();
		}) as Box<dyn FnMut(_)>);
		channel.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
		on_message.forget();
		Ok(channel)
	}

	/// Close watcher channel if no subscriptions.
	fn maybe_stop_watch(state: &mut IdbLocalsState) {
		if state.watchers.is_empty() {
			if let Some(channel) = take(&mut state.channel) {
				channel.close();
			}
		}
	}
}
impl LocalActor for IdbLocalsActor {
	type Message = IdbLocalsMessage;
	type State = IdbLocalsState;
	type Initialize = String;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &co_primitives::Tags,
		db_name: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		let db = open_database(&db_name)
			.await
			.map_err(|e| ActorError::Actor(anyhow!("IndexedDB open failed: {:?}", e)))?;
		Ok(IdbLocalsState { db_name, db, channel: None, watchers: Default::default() })
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			IdbLocalsMessage::Get(response) => response.respond(Self::handle_get(&state.db).await),
			IdbLocalsMessage::Set(local, response) => response.respond(Self::handle_set(state, local).await),
			IdbLocalsMessage::Watch(stream) => {
				// add
				state.watchers.push(stream);

				// start watch
				state.channel = Some(Self::start_watch(handle.clone(), &state.db_name)?);
			},
			IdbLocalsMessage::Notify => Self::handle_notify(state).await,
		}
		Ok(())
	}

	async fn shutdown(&self, mut state: Self::State) -> Result<(), ActorError> {
		if let Some(channel) = take(&mut state.channel) {
			channel.close();
		}
		Ok(())
	}
}

/// Open (or upgrade-create) an IndexedDB database.
async fn open_database(name: &str) -> Result<IdbDatabase, JsValue> {
	let factory: IdbFactory = web_sys::window()
		.ok_or_else(|| JsValue::from_str("no window"))?
		.indexed_db()?
		.ok_or_else(|| JsValue::from_str("indexedDB not available"))?;

	let open_request: IdbOpenDbRequest = factory.open_with_u32(name, DB_VERSION)?;

	let on_upgrade = Closure::once(move |event: web_sys::Event| {
		let request: IdbOpenDbRequest = event.target().unwrap().dyn_into::<IdbOpenDbRequest>().unwrap();
		let db: IdbDatabase = request.result().unwrap().dyn_into::<IdbDatabase>().unwrap();
		if !db.object_store_names().contains(OBJECT_STORE_NAME) {
			db.create_object_store(OBJECT_STORE_NAME).unwrap();
		}
	});
	open_request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));

	let db_js = idb_request_await(open_request.as_ref())
		.await
		.map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

	Ok(db_js.dyn_into::<IdbDatabase>()?)
}

/// Await an IdbRequest by wrapping its onsuccess/onerror in a Promise.
async fn idb_request_await(request: &IdbRequest) -> Result<JsValue, anyhow::Error> {
	let promise = js_sys::Promise::new(&mut |resolve, reject| {
		let resolve_cb = Closure::once(move |event: web_sys::Event| {
			let target: IdbRequest = event.target().unwrap().dyn_into().unwrap();
			resolve.call1(&JsValue::NULL, &target.result().unwrap()).unwrap();
		});
		let reject_cb = Closure::once(move |event: web_sys::Event| {
			let target: IdbRequest = event.target().unwrap().dyn_into().unwrap();
			let err: JsValue = target
				.error()
				.ok()
				.flatten()
				.map(JsValue::from)
				.unwrap_or_else(|| JsValue::from_str("unknown IDB error"));
			reject.call1(&JsValue::NULL, &err).unwrap();
		});
		request.set_onsuccess(Some(resolve_cb.as_ref().unchecked_ref()));
		request.set_onerror(Some(reject_cb.as_ref().unchecked_ref()));
		resolve_cb.forget();
		reject_cb.forget();
	});
	JsFuture::from(promise)
		.await
		.map_err(|e| anyhow!("IDB request failed: {:?}", e))
}

/// Convert a JS value (Uint8Array or ArrayBuffer) to `Vec<u8>`.
fn bytes_from_js(value: &JsValue) -> Result<Vec<u8>, JsValue> {
	if let Ok(arr) = value.clone().dyn_into::<js_sys::Uint8Array>() {
		return Ok(arr.to_vec());
	}
	if let Ok(buf) = value.clone().dyn_into::<js_sys::ArrayBuffer>() {
		return Ok(js_sys::Uint8Array::new(&buf).to_vec());
	}
	Err(JsValue::from_str("expected Uint8Array or ArrayBuffer"))
}

/// Convert a byte slice to a JS `Uint8Array`.
fn bytes_to_js(data: &[u8]) -> JsValue {
	js_sys::Uint8Array::from(data).into()
}
