// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_messaging::{
	multimedia::{ImageInfo, ThumbnailInfo},
	state_event, MatrixEvent,
};

#[test]
fn room_name() {
	let content = state_event::RoomNameContent::new("Some name");
	let event = MatrixEvent::new("event1234", 5000, "$some:room", content);
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap());

	let thumbnail_info = ThumbnailInfo::new(10, 10, "image/png", 1000);
	let image_info = ImageInfo::new(100, 100, "image/png", 10000, Cid::default(), thumbnail_info);
	state_event::RoomAvatarContent::new(state_event::Avatar::Image { cid: Cid::default(), info: image_info });
}
