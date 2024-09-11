use libipld::Cid;
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

/**
 * Contains metadata of images
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct ImageInfo {
	pub h: u32,                        // intended display height in px
	pub w: u32,                        // intended display width in px
	pub mimetype: String,              // mimetype of the file
	pub size: u32,                     // Size of the image file in bytes
	pub thumbnail_file: Cid,           // CID to an image file that is to be used as the thumbnail
	pub thumbnail_info: ThumbnailInfo, // thumbnail metadata
}

/**
 * Contains metadata of images used as a thumbnail
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct ThumbnailInfo {
	pub h: u32,           // intended display height in px
	pub w: u32,           // intended display width in px
	pub mimetype: String, // mimetype of the file
	pub size: u32,        // Size of the image file in bytes
}

/**
 * Contains metadata of audio files
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct AudioInfo {
	#[typeshare(retype = "number")]
	pub duration: u32, // duration of the audio clip in ms
	pub mimetype: String, // mimetype of the audio file
	pub size: u32,        // size of the audio file in bytes
}

/**
 * Contains metadata of video files
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct VideoInfo {
	pub h: u32,                        // intended display height in px
	pub w: u32,                        // intended display width in px
	pub duration: u32,                 // duration of the video clip in ms
	pub mimetype: String,              // mimetype of the file
	pub size: u32,                     // Size of the image file in bytes
	pub thumbnail_file: Cid,           // CID to an image file that is to be used as the thumbnail
	pub thumbnail_info: ThumbnailInfo, // thumbnail metadata
}

/**
 * Contains metadata of any other filetypes
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct FileInfo {
	pub mimetype: String,              // mimetype of the file
	pub size: u32,                     // Size of the file in bytes
	pub thumbnail_file: Cid,           // CID to an image file that is to be used as the thumbnail
	pub thumbnail_info: ThumbnailInfo, // thumbnail metadata
}

/**
 * Contains metadata of any location based content
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct LocationInfo {
	pub thumbnail_file: Cid,           // CID to an image file that is to be used as the thumbnail
	pub thumbnail_info: ThumbnailInfo, // thumbnail metadata
}
