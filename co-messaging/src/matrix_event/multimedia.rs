// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_macros::co;
use co_primitives::CoCid;
use schemars::JsonSchema;

/// Contains metadata of images
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct ImageInfo {
	/// Intended display height in px
	pub h: u32,
	/// Intended display width in px
	pub w: u32,
	/// Mimetype of the file
	pub mimetype: String,
	/// Size of the image file in bytes
	pub size: u32,
	/// CID to an image file that is to be used as the thumbnail
	#[schemars(with = "CoCid")]
	pub thumbnail_file: Cid,
	/// Thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}

impl ImageInfo {
	pub fn new(
		h: u32,
		w: u32,
		mimetype: impl Into<String>,
		size: u32,
		thumbnail_file: Cid,
		thumbnail_info: ThumbnailInfo,
	) -> Self {
		Self { h, w, mimetype: mimetype.into(), size, thumbnail_file, thumbnail_info }
	}
}

/// Contains metadata of images used as a thumbnail
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct ThumbnailInfo {
	/// Intended display height in px
	pub h: u32,
	/// Intended display width in px
	pub w: u32,
	/// Mimetype of the file
	pub mimetype: String,
	/// Size of the image file in bytes
	pub size: u32,
}

impl ThumbnailInfo {
	pub fn new(h: u32, w: u32, mimetype: impl Into<String>, size: u32) -> Self {
		Self { h, w, mimetype: mimetype.into(), size }
	}
}

/// Contains metadata of audio files
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct AudioInfo {
	/// Duration of the audio clip in ms
	pub duration: u32,
	/// Mimetype of the audio file
	pub mimetype: String,
	/// Size of the audio file in bytes
	pub size: u32,
	/// Vector with data for the waveform visualisation. Values from 0 to 256 are possible.
	/// The entries in the vector should be distributed in a linear fashion.
	/// Not in the official specs yet, but introduced [in this proposal](https://github.com/matrix-org/matrix-spec-proposals/pull/3246)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub waveform: Option<Vec<u8>>,
}

impl AudioInfo {
	pub fn new(duration: u32, mimetype: impl Into<String>, size: u32) -> Self {
		Self { duration, mimetype: mimetype.into(), size, ..Default::default() }
	}
}

/// Contains metadata of video files
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct VideoInfo {
	/// Intended display height in px
	pub h: u32,
	/// Intended display width in px
	pub w: u32,
	/// Duration of the video clip in ms
	pub duration: u32,
	/// Mimetype of the file
	pub mimetype: String,
	/// Size of the image file in bytes
	pub size: u32,
	/// CID to an image file that is to be used as the thumbnail
	#[schemars(with = "CoCid")]
	pub thumbnail_file: Cid,
	/// Thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}

impl VideoInfo {
	pub fn new(
		h: u32,
		w: u32,
		duration: u32,
		mimetype: impl Into<String>,
		size: u32,
		thumbnail_file: Cid,
		thumbnail_info: ThumbnailInfo,
	) -> Self {
		Self { h, w, duration, mimetype: mimetype.into(), size, thumbnail_file, thumbnail_info }
	}
}

/// Contains metadata of any other filetypes
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct FileInfo {
	/// Mimetype of the file
	pub mimetype: String,
	/// Size of the file in bytes
	pub size: u32,
	/// CID to an image file that is to be used as the thumbnail
	#[schemars(with = "CoCid")]
	pub thumbnail_file: Cid,
	/// Thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}

impl FileInfo {
	pub fn new(mimetype: impl Into<String>, size: u32, thumbnail_file: Cid, thumbnail_info: ThumbnailInfo) -> Self {
		Self { mimetype: mimetype.into(), size, thumbnail_file, thumbnail_info }
	}
}

/// Contains metadata of any location based content
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct LocationInfo {
	/// CID to an image file that is to be used as the thumbnail
	#[schemars(with = "CoCid")]
	pub thumbnail_file: Cid,
	/// Thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}

impl LocationInfo {
	pub fn new(thumbnail_file: Cid, thumbnail_info: ThumbnailInfo) -> Self {
		Self { thumbnail_file, thumbnail_info }
	}
}
