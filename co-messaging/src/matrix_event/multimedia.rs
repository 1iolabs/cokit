// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_macros::co_data;
use co_primitives::CoCid;
use schemars::JsonSchema;

/// Contains metadata of images
#[co_data]
#[derive(JsonSchema)]
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

/// Contains metadata of images used as a thumbnail
#[co_data]
#[derive(JsonSchema)]
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

/// Contains metadata of audio files
#[co_data]
#[derive(JsonSchema)]
pub struct AudioInfo {
	/// Duration of the audio clip in ms
	pub duration: u32,
	/// Mimetype of the audio file
	pub mimetype: String,
	/// Size of the audio file in bytes
	pub size: u32,
}

/// Contains metadata of video files
#[co_data]
#[derive(JsonSchema)]
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

/// Contains metadata of any other filetypes
#[co_data]
#[derive(JsonSchema)]
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

/// Contains metadata of any location based content
#[co_data]
#[derive(JsonSchema)]
pub struct LocationInfo {
	/// CID to an image file that is to be used as the thumbnail
	#[schemars(with = "CoCid")]
	pub thumbnail_file: Cid,
	/// Thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}
