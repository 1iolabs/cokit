// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_messaging::{multimedia::ImageInfo, state_event, MatrixEvent};

#[test]
fn room_name() {
	let content = state_event::RoomNameContent::new("Some name");
	let event = MatrixEvent::new("event1234", 5000, "$some:room", content);
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap());

	state_event::RoomAvatarContent::new(
		Some(Cid::default()),
		ImageInfo {
			h: 100,
			w: 100,
			size: 10000,
			mimetype: "image/png".into(),
			thumbnail_info: co_messaging::multimedia::ThumbnailInfo {
				h: 10,
				w: 10,
				mimetype: "image/png".into(),
				size: 1000,
			},
			thumbnail_file: Default::default(),
		},
	);
}
