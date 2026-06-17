// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_messaging::{multimedia::ImageInfo, state_event, MatrixEvent};

#[test]
fn room_name() {
	let content = state_event::RoomNameContent::new("Some name");
	let event = MatrixEvent::new("event1234", 5000, "$some:room", content);
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap());

	let mut thumbnail_info = co_messaging::multimedia::ThumbnailInfo::default();
	thumbnail_info.h = 10;
	thumbnail_info.w = 10;
	thumbnail_info.mimetype = "image/png".into();
	thumbnail_info.size = 1000;
	let mut image_info = ImageInfo::default();
	image_info.h = 100;
	image_info.w = 100;
	image_info.size = 10000;
	image_info.mimetype = "image/png".into();
	image_info.thumbnail_info = thumbnail_info;
	state_event::RoomAvatarContent::new(Some(Cid::default()), image_info);
}
