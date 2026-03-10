// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_api::{
	async_api::Reducer, co, BlockStorage, BlockStorageExt, CoList, CoListIndex, CoMap, CoreBlockStorage, IsDefault,
	Link, OptionLink, ReducerAction, Tags,
};
pub use co_messaging::MatrixEvent;
use co_messaging::{message_event::MessageType, relation::Relation, EventContent};
use co_primitives::CoCid;
use schemars::JsonSchema;

/// Pre-calculated room event for direct display without event sourcing.
#[co]
pub struct RoomEvent {
	/// event_id for identification and index lookup
	#[serde(rename = "i")]
	pub id: String,

	/// current version of this event's action (updated to edit action on edit)
	#[serde(rename = "e")]
	pub event: Link<ReducerAction<MatrixEvent>>,

	/// link to the replied-to message's action
	#[serde(rename = "p", skip_serializing_if = "Option::is_none")]
	pub reply_to: Option<Link<ReducerAction<MatrixEvent>>>,

	/// reactions, deduplicated per sender (last-reaction-wins)
	#[serde(rename = "r", default, skip_serializing_if = "Vec::is_empty")]
	pub reactions: Vec<Link<ReducerAction<MatrixEvent>>>,

	/// edit history (previous event links, newest last)
	#[serde(rename = "h", default, skip_serializing_if = "Vec::is_empty")]
	pub edits: Vec<Link<ReducerAction<MatrixEvent>>>,

	/// modification events: poll votes, poll closes, checklist adds/checks
	#[serde(rename = "m", default, skip_serializing_if = "Vec::is_empty")]
	pub modifications: Vec<Link<ReducerAction<MatrixEvent>>>,

	/// redacted
	#[serde(rename = "d", default, skip_serializing_if = "IsDefault::is_default")]
	pub is_deleted: bool,

	/// closed (poll or checklist)
	#[serde(rename = "c", default, skip_serializing_if = "IsDefault::is_default")]
	pub is_closed: bool,
}

#[co(state)]
#[derive(JsonSchema)]
pub struct Room {
	pub name: String,
	pub description: String,
	#[schemars(with = "Option<CoCid>")]
	pub avatar: Option<Cid>,
	pub pinned_messages: Vec<String>,
	pub tags: Tags,

	/// Ordered events for display
	#[schemars(skip)]
	pub events: CoList<Link<RoomEvent>>,

	/// Index from event_id to CoListIndex
	#[schemars(skip)]
	pub event_index: CoMap<String, CoListIndex>,
}
impl Reducer<MatrixEvent> for Room {
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<MatrixEvent>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let event: ReducerAction<MatrixEvent> = storage.get_value(&event_link).await?;
		let mut state: Room = storage.get_value_or_default(&state_link).await?;

		// apply room state changes
		match &event.payload.content {
			EventContent::RoomName(c) => state.name = c.name.clone(),
			EventContent::RoomTopic(c) => state.description = c.topic.clone(),
			EventContent::RoomAvatar(c) => state.avatar = c.file,
			EventContent::PinnedEvents(c) => state.pinned_messages = c.pinned.clone(),
			_ => {},
		}

		// apply event list changes
		reduce_events(storage, &mut state, &event.payload, event_link).await?;

		Ok(storage.set_value(&state).await?)
	}
}

async fn reduce_events<S>(
	storage: &S,
	state: &mut Room,
	matrix_event: &MatrixEvent,
	event_link: Link<ReducerAction<MatrixEvent>>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	match &matrix_event.content {
		EventContent::Message(msg_type) => reduce_message(storage, state, matrix_event, msg_type, event_link).await,
		EventContent::Reaction(reaction) => {
			let target_id = reaction.relates_to.as_ref().and_then(|r| r.event_id.clone());
			if let Some(target_id) = target_id {
				update_room_event_simple(storage, state, &target_id, |room_event| {
					room_event.reactions.push(event_link);
				})
				.await
			} else {
				Ok(())
			}
		},
		EventContent::Redaction(redaction) => {
			update_room_event_simple(storage, state, &redaction.redacts, |room_event| {
				room_event.is_deleted = true;
				room_event.reactions.clear();
			})
			.await
		},
		// state events, ephemeral, calls — skip
		_ => Ok(()),
	}
}

async fn reduce_message<S>(
	storage: &S,
	state: &mut Room,
	matrix_event: &MatrixEvent,
	msg_type: &MessageType,
	event_link: Link<ReducerAction<MatrixEvent>>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	// skip control notices (read receipts, typing indicators)
	if let MessageType::Notice(nc) = msg_type {
		if nc.body.starts_with("__READ_RECEIPT__") || nc.body == "__TYPING__" {
			return Ok(());
		}
		// checklist item addition — attach to target
		if nc.body.starts_with("__CHECKLIST_ADD__") {
			if let Some(checklist_id) = nc.body["__CHECKLIST_ADD__".len()..].split_once('\n').map(|(id, _)| id) {
				return update_room_event_simple(storage, state, checklist_id, |room_event| {
					room_event.modifications.push(event_link);
				})
				.await;
			}
			return Ok(());
		}
	}

	// poll vote — attach to target
	if let MessageType::Response(prc) = msg_type {
		if let Some(target_id) = prc.relates_to.as_ref().and_then(|r| r.event_id.as_ref()) {
			return update_room_event_simple(storage, state, target_id, |room_event| {
				room_event.modifications.push(event_link);
			})
			.await;
		}
		return Ok(());
	}

	// poll/checklist close — attach to target + set is_closed
	if let MessageType::End(pec) = msg_type {
		if let Some(target_id) = pec.relates_to.as_ref().and_then(|r| r.event_id.as_ref()) {
			return update_room_event_simple(storage, state, target_id, |room_event| {
				room_event.is_closed = true;
				room_event.modifications.push(event_link);
			})
			.await;
		}
		return Ok(());
	}

	// edit (replacement) — update target in-place
	let relation_type = msg_type.generate_relation_type();
	let is_replace = relation_type.as_deref() == Some("m.replace");
	if is_replace {
		if let Some(target_id) = get_relates_to_event_id(msg_type) {
			return update_room_event_simple(storage, state, &target_id, |room_event| {
				// push current event to edit history, set event to the new edit
				room_event.edits.push(room_event.event);
				room_event.event = event_link;
			})
			.await;
		}
		return Ok(());
	}

	// new message — create RoomEvent and push to list
	let reply_to = resolve_reply_to(storage, state, msg_type).await?;
	let room_event = RoomEvent {
		id: matrix_event.event_id.clone(),
		event: event_link,
		reply_to,
		reactions: Vec::new(),
		edits: Vec::new(),
		modifications: Vec::new(),
		is_deleted: false,
		is_closed: false,
	};
	let room_event_link: Link<RoomEvent> = storage.set_value(&room_event).await?;
	let idx = state.events.push(storage, room_event_link).await?;
	state.event_index.insert(storage, matrix_event.event_id.clone(), idx).await?;
	Ok(())
}

/// Resolve the reply_to link for a new message.
async fn resolve_reply_to<S>(
	storage: &S,
	state: &Room,
	msg_type: &MessageType,
) -> Result<Option<Link<ReducerAction<MatrixEvent>>>, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let reply_event_id = msg_type.get_in_reply_to();
	let reply_event_id = match reply_event_id {
		Some(id) => id,
		None => return Ok(None),
	};

	// look up the target in event_index
	let idx = state.event_index.get(storage, &reply_event_id).await?;
	let idx = match idx {
		Some(idx) => idx,
		None => return Ok(None),
	};

	let room_event_link = state.events.get(storage, &idx).await?;
	let room_event_link = match room_event_link {
		Some(link) => link,
		None => return Ok(None),
	};

	let room_event: RoomEvent = storage.get_value(&room_event_link).await?;
	Ok(Some(room_event.event))
}

/// Update an existing RoomEvent in the events list.
async fn update_room_event_simple<S, F>(
	storage: &S,
	state: &mut Room,
	target_id: &str,
	update: F,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
	F: FnOnce(&mut RoomEvent),
{
	let idx = match state.event_index.get(storage, &target_id.to_string()).await? {
		Some(idx) => idx,
		None => return Ok(()),
	};
	let room_event_link = match state.events.get(storage, &idx).await? {
		Some(link) => link,
		None => return Ok(()),
	};
	let mut room_event: RoomEvent = storage.get_value(&room_event_link).await?;
	update(&mut room_event);
	let new_link: Link<RoomEvent> = storage.set_value(&room_event).await?;
	state.events.set(storage, idx, new_link).await?;
	Ok(())
}

/// Extract the target event_id from a message's relates_to.
fn get_relates_to_event_id(msg_type: &MessageType) -> Option<String> {
	match msg_type {
		MessageType::Text(c) => c.relates_to.as_ref()?.event_id.clone(),
		MessageType::Notice(c) => c.relates_to.as_ref()?.event_id.clone(),
		MessageType::Image(c) => c.relates_to.as_ref()?.event_id.clone(),
		MessageType::Audio(c) => c.relates_to.as_ref()?.event_id.clone(),
		MessageType::Video(c) => c.relates_to.as_ref()?.event_id.clone(),
		MessageType::File(c) => c.relates_to.as_ref()?.event_id.clone(),
		MessageType::Location(c) => c.relates_to.as_ref()?.event_id.clone(),
		MessageType::Start(c) => c.relates_to.as_ref()?.event_id.clone(),
		MessageType::Response(c) => c.relates_to.as_ref()?.event_id.clone(),
		MessageType::End(c) => c.relates_to.as_ref()?.event_id.clone(),
	}
}
