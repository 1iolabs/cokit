use co_api::{
	sync_api::{Context, Reducer},
	ReducerAction, Tags,
};
use co_messaging::{state_event::StateType, EventContent, MatrixEvent};
use co_primitives::CoCid;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/**
 * eco Messenger room COre
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct Room {
	/// Name of the room
	pub name: String,

	/// A short description for the room
	pub description: String,

	/// Content ID for the room avatar
	pub avatar: Option<CoCid>,

	/// All currently pinned messages in relevant order
	pub pinned_messages: Vec<String>,

	pub tags: Tags,
}

impl Reducer for Room {
	type Action = MatrixEvent;

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &mut dyn Context) -> Self {
		let matrix_event = &event.payload;

		let mut result = self.clone();
		match &matrix_event.content {
			EventContent::State(state_content) => match state_content {
				StateType::RoomName(name_content) => result.name = name_content.name.clone(),
				StateType::RoomTopic(topic_content) => result.description = topic_content.topic.clone(),
				StateType::RoomAvatar(avatar_content) => result.avatar = avatar_content.file,
				StateType::PinnedEvents(pin_content) => result.pinned_messages = pin_content.pinned.clone(),
			},
			_ => (),
		};
		result
	}
}

#[cfg(all(feature = "core", target_arch = "wasm32", target_os = "unknown"))]
#[no_mangle]
pub extern "C" fn state() {
	co_api::sync_api::reduce::<Room>()
}
