use co_primitives::CoCid;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

/**
 * Contains metadata of images
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, JsonSchema)]
pub struct ImageInfo {
	/// intended display height in px
	pub h: u32,
	/// intended display width in px
	pub w: u32,
	/// mimetype of the file
	pub mimetype: String,
	/// Size of the image file in bytes
	pub size: u32,
	/// CID to an image file that is to be used as the thumbnail
	pub thumbnail_file: CoCid,
	/// thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}

/**
 * Contains metadata of images used as a thumbnail
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, schemars::JsonSchema)]
pub struct ThumbnailInfo {
	/// intended display height in px
	pub h: u32,
	/// intended display width in px
	pub w: u32,
	/// mimetype of the file
	pub mimetype: String,
	/// Size of the image file in bytes
	pub size: u32,
}

/**
 * Contains metadata of audio files
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct AudioInfo {
	#[typeshare(retype = "number")]
	/// duration of the audio clip in ms
	pub duration: u32,
	/// mimetype of the audio file
	pub mimetype: String,
	/// size of the audio file in bytes
	pub size: u32,
}

/**
 * Contains metadata of video files
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct VideoInfo {
	/// intended display height in px
	pub h: u32,
	/// intended display width in px
	pub w: u32,
	/// duration of the video clip in ms
	pub duration: u32,
	/// mimetype of the file
	pub mimetype: String,
	/// Size of the image file in bytes
	pub size: u32,
	/// CID to an image file that is to be used as the thumbnail
	pub thumbnail_file: CoCid,
	/// thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}

/**
 * Contains metadata of any other filetypes
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct FileInfo {
	/// mimetype of the file
	pub mimetype: String,
	/// Size of the file in bytes
	pub size: u32,
	/// CID to an image file that is to be used as the thumbnail
	pub thumbnail_file: CoCid,
	/// thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}

/**
 * Contains metadata of any location based content
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct LocationInfo {
	/// CID to an image file that is to be used as the thumbnail
	pub thumbnail_file: CoCid,
	/// thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}
