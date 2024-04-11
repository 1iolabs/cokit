use co_api::{reduce, Context, Reducer, ReducerAction, Tags};
use co_messaging::{state_event::StateType, EventContent, MatrixEvent};
use libipld::Cid;
use serde::{Deserialize, Serialize};

/**
 * eco Messenger room COre
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Room {
	/// Name of the room
	pub name: String,

	/// A short description for the room
	pub description: String,

	/// Content ID for the room avatar
	pub avatar: Cid,

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
				StateType::RoomAvatar(avatar_content) => result.avatar = avatar_content.file.clone(),
				StateType::PinnedEvents(pin_content) => result.pinned_messages = pin_content.pinned.clone(),
			},
			_ => (),
		};
		result
	}
}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<Room>()
}
