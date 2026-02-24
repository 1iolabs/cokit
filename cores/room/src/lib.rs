// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_api::{
	sync_api::{Context, Reducer},
	ReducerAction, Tags,
};
use co_messaging::{EventContent, MatrixEvent};
use co_primitives::CoCid;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// eCO Messenger room core
#[derive(Debug, Clone, Default, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct Room {
	/// Name of the room
	pub name: String,

	/// A short description for the room
	pub description: String,

	/// Content ID for the room avatar
	#[schemars(with = "Option<CoCid>")]
	pub avatar: Option<Cid>,

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
			EventContent::RoomName(name_content) => result.name = name_content.name.clone(),
			EventContent::RoomTopic(topic_content) => result.description = topic_content.topic.clone(),
			EventContent::RoomAvatar(avatar_content) => result.avatar = avatar_content.file,
			EventContent::PinnedEvents(pin_content) => result.pinned_messages = pin_content.pinned.clone(),
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
