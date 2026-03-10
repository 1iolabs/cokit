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

#[cfg(test)]
mod tests {
	use super::*;
	use co_api::{async_api::Reducer, BlockStorageExt, CoreBlockStorage, Date, ReducerAction};
	use co_messaging::{
		message_event::{NoticeContent, TextContent},
		poll_event::{PollAnswer, PollEndContent, PollKind, PollResponseContent, PollStartContent},
		relation::{ReactionContent, RedactionContent, RelatesTo},
		state_event::{RoomNameContent, RoomTopicContent},
		EventContent, MatrixEvent,
	};
	use co_storage::MemoryBlockStorage;

	async fn dispatch(
		storage: &MemoryBlockStorage,
		time: &mut Date,
		state: Room,
		from: &str,
		event_id: &str,
		content: impl Into<EventContent>,
	) -> Room {
		let event = MatrixEvent::new(event_id, *time, "room1", content);
		let action = ReducerAction { core: "".to_owned(), from: from.to_owned(), payload: event, time: *time };
		*time += 1;
		let action_link = storage.set_value(&action).await.unwrap();
		let state_link = storage.set_value(&state).await.unwrap();
		let next_link = Room::reduce(state_link.into(), action_link, &CoreBlockStorage::new(storage.clone(), true))
			.await
			.unwrap();
		storage.get_value(&next_link).await.unwrap()
	}

	async fn get_event(storage: &MemoryBlockStorage, state: &Room, event_id: &str) -> Option<RoomEvent> {
		let idx = state.event_index.get(storage, &event_id.to_string()).await.unwrap()?;
		let link = state.events.get(storage, &idx).await.unwrap()?;
		Some(storage.get_value(&link).await.unwrap())
	}

	async fn event_count(storage: &MemoryBlockStorage, state: &Room) -> usize {
		state.events.vec(storage, None).await.unwrap().len()
	}

	#[tokio::test]
	async fn new_text_message() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$msg1", TextContent::new("hello")).await;

		assert_eq!(event_count(&storage, &state).await, 1);
		let ev = get_event(&storage, &state, "$msg1").await.unwrap();
		assert_eq!(ev.id, "$msg1");
		assert!(!ev.is_deleted);
		assert!(!ev.is_closed);
		assert!(ev.reactions.is_empty());
		assert!(ev.edits.is_empty());
		assert!(ev.modifications.is_empty());
		assert!(ev.reply_to.is_none());
	}

	#[tokio::test]
	async fn room_name_and_topic() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$e1", RoomNameContent::new("My Room")).await;
		assert_eq!(state.name, "My Room");
		assert_eq!(event_count(&storage, &state).await, 0);

		let state = dispatch(&storage, &mut time, state, "alice", "$e2", RoomTopicContent::new("A topic")).await;
		assert_eq!(state.description, "A topic");
		assert_eq!(event_count(&storage, &state).await, 0);
	}

	#[tokio::test]
	async fn edit_updates_in_place() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$msg1", TextContent::new("original")).await;
		let original_event = get_event(&storage, &state, "$msg1").await.unwrap();
		let original_link = original_event.event;

		// send edit
		let mut edit = TextContent::new("edited");
		edit.relates_to = Some(RelatesTo::replacement("$msg1"));
		let state = dispatch(&storage, &mut time, state, "alice", "$edit1", edit).await;

		// still one event in the list
		assert_eq!(event_count(&storage, &state).await, 1);
		let ev = get_event(&storage, &state, "$msg1").await.unwrap();
		// event link should point to the edit now
		assert_ne!(ev.event, original_link);
		// edit history should contain the original
		assert_eq!(ev.edits.len(), 1);
		assert_eq!(ev.edits[0], original_link);
	}

	#[tokio::test]
	async fn reaction_attaches_to_target() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$msg1", TextContent::new("hi")).await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			"bob",
			"$react1",
			ReactionContent::new(RelatesTo::annotation("$msg1", "👍")),
		)
		.await;

		assert_eq!(event_count(&storage, &state).await, 1);
		let ev = get_event(&storage, &state, "$msg1").await.unwrap();
		assert_eq!(ev.reactions.len(), 1);
	}

	#[tokio::test]
	async fn redaction_marks_deleted_and_clears_reactions() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$msg1", TextContent::new("hi")).await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			"bob",
			"$react1",
			ReactionContent::new(RelatesTo::annotation("$msg1", "👍")),
		)
		.await;

		// redact
		let state =
			dispatch(&storage, &mut time, state, "alice", "$redact1", RedactionContent::new("$msg1", None)).await;

		let ev = get_event(&storage, &state, "$msg1").await.unwrap();
		assert!(ev.is_deleted);
		assert!(ev.reactions.is_empty());
	}

	#[tokio::test]
	async fn reply_resolves_link() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$msg1", TextContent::new("hello")).await;
		let original_event = get_event(&storage, &state, "$msg1").await.unwrap();

		let mut reply = TextContent::new("reply");
		reply.relates_to = Some(RelatesTo::in_reply_to("$msg1"));
		let state = dispatch(&storage, &mut time, state, "bob", "$reply1", reply).await;

		assert_eq!(event_count(&storage, &state).await, 2);
		let reply_ev = get_event(&storage, &state, "$reply1").await.unwrap();
		assert_eq!(reply_ev.reply_to.unwrap(), original_event.event);
	}

	#[tokio::test]
	async fn skip_read_receipt_and_typing() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state =
			dispatch(&storage, &mut time, state, "alice", "$e1", NoticeContent::new("__READ_RECEIPT__123")).await;
		assert_eq!(event_count(&storage, &state).await, 0);

		let state = dispatch(&storage, &mut time, state, "alice", "$e2", NoticeContent::new("__TYPING__")).await;
		assert_eq!(event_count(&storage, &state).await, 0);
	}

	#[tokio::test]
	async fn regular_notice_creates_event() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$n1", NoticeContent::new("a notice")).await;
		assert_eq!(event_count(&storage, &state).await, 1);
		let ev = get_event(&storage, &state, "$n1").await.unwrap();
		assert_eq!(ev.id, "$n1");
	}

	#[tokio::test]
	async fn checklist_add_attaches_to_target() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$cl1", TextContent::new("checklist")).await;
		let state =
			dispatch(&storage, &mut time, state, "bob", "$add1", NoticeContent::new("__CHECKLIST_ADD__$cl1\nnew item"))
				.await;

		// should not create a new event in the list
		assert_eq!(event_count(&storage, &state).await, 1);
		let ev = get_event(&storage, &state, "$cl1").await.unwrap();
		assert_eq!(ev.modifications.len(), 1);
	}

	#[tokio::test]
	async fn poll_vote_attaches_to_target() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let poll = PollStartContent::new(
			"Favorite color?",
			vec![PollAnswer::new("1", "Red"), PollAnswer::new("2", "Blue")],
			PollKind::Disclosed,
		);
		let state = dispatch(&storage, &mut time, state, "alice", "$poll1", poll).await;

		let vote = PollResponseContent::new("vote", vec!["1".to_owned()], "$poll1");
		let state = dispatch(&storage, &mut time, state, "bob", "$vote1", vote).await;

		assert_eq!(event_count(&storage, &state).await, 1);
		let ev = get_event(&storage, &state, "$poll1").await.unwrap();
		assert_eq!(ev.modifications.len(), 1);
		assert!(!ev.is_closed);
	}

	#[tokio::test]
	async fn poll_close_sets_is_closed() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let poll = PollStartContent::new(
			"Question?",
			vec![PollAnswer::new("1", "Yes"), PollAnswer::new("2", "No")],
			PollKind::Disclosed,
		);
		let state = dispatch(&storage, &mut time, state, "alice", "$poll1", poll).await;

		let close = PollEndContent::new("Poll ended", "$poll1");
		let state = dispatch(&storage, &mut time, state, "alice", "$close1", close).await;

		let ev = get_event(&storage, &state, "$poll1").await.unwrap();
		assert!(ev.is_closed);
		assert_eq!(ev.modifications.len(), 1);
	}

	#[tokio::test]
	async fn multiple_edits_build_history() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$msg1", TextContent::new("v1")).await;

		let mut edit1 = TextContent::new("v2");
		edit1.relates_to = Some(RelatesTo::replacement("$msg1"));
		let state = dispatch(&storage, &mut time, state, "alice", "$edit1", edit1).await;

		let mut edit2 = TextContent::new("v3");
		edit2.relates_to = Some(RelatesTo::replacement("$msg1"));
		let state = dispatch(&storage, &mut time, state, "alice", "$edit2", edit2).await;

		let ev = get_event(&storage, &state, "$msg1").await.unwrap();
		assert_eq!(ev.edits.len(), 2);
		assert_eq!(event_count(&storage, &state).await, 1);
	}

	#[tokio::test]
	async fn multiple_reactions_on_same_message() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(&storage, &mut time, state, "alice", "$msg1", TextContent::new("hi")).await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			"bob",
			"$r1",
			ReactionContent::new(RelatesTo::annotation("$msg1", "👍")),
		)
		.await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			"carol",
			"$r2",
			ReactionContent::new(RelatesTo::annotation("$msg1", "❤️")),
		)
		.await;

		let ev = get_event(&storage, &state, "$msg1").await.unwrap();
		assert_eq!(ev.reactions.len(), 2);
	}

	#[tokio::test]
	async fn reaction_on_nonexistent_target_is_noop() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state = dispatch(
			&storage,
			&mut time,
			state,
			"bob",
			"$r1",
			ReactionContent::new(RelatesTo::annotation("$nonexistent", "👍")),
		)
		.await;

		assert_eq!(event_count(&storage, &state).await, 0);
	}

	#[tokio::test]
	async fn edit_on_nonexistent_target_is_noop() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let mut edit = TextContent::new("edited");
		edit.relates_to = Some(RelatesTo::replacement("$nonexistent"));
		let state = dispatch(&storage, &mut time, state, "alice", "$edit1", edit).await;

		assert_eq!(event_count(&storage, &state).await, 0);
	}

	#[tokio::test]
	async fn redaction_on_nonexistent_target_is_noop() {
		let storage = MemoryBlockStorage::default();
		let mut time: Date = 1000;
		let state = Room::default();

		let state =
			dispatch(&storage, &mut time, state, "alice", "$redact1", RedactionContent::new("$nonexistent", None))
				.await;

		assert_eq!(event_count(&storage, &state).await, 0);
	}
}
