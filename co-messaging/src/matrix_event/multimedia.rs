use libipld::Cid;
use serde::{Deserialize, Serialize};

/**
 * Contains metadata of images
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ImageInfo {
	pub h: i64,                        // intended display height in px
	pub w: i64,                        // intended display width in px
	pub mimetype: String,              // mimetype of the file
	pub size: i64,                     // Size of the image file in bytes
	pub thumbnail_file: Cid,           // CID to an image file that is to be used as the thumbnail
	pub thumbnail_info: ThumbnailInfo, // thumbnail metadata
}

/**
 * Contains metadata of images used as a thumbnail
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ThumbnailInfo {
	pub h: i64,           // intended display height in px
	pub w: i64,           // intended display width in px
	pub mimetype: String, // mimetype of the file
	pub size: i64,        // Size of the image file in bytes
}

/**
 * Contains metadata of audio files
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct AudioInfo {
	pub duration: i64,    // duration of the audio clip in ms
	pub mimetype: String, // mimetype of the audio file
	pub size: i64,        // size of the audio file in bytes
}

/**
 * Contains metadata of video files
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct VideoInfo {
	pub h: i64,                        // intended display height in px
	pub w: i64,                        // intended display width in px
	pub duration: i64,                 // duration of the video clip in ms
	pub mimetype: String,              // mimetype of the file
	pub size: i64,                     // Size of the image file in bytes
	pub thumbnail_file: Cid,           // CID to an image file that is to be used as the thumbnail
	pub thumbnail_info: ThumbnailInfo, // thumbnail metadata
}

/**
 * Contains metadata of any other filetypes
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct FileInfo {
	pub mimetype: String,              // mimetype of the file
	pub size: i64,                     // Size of the file in bytes
	pub thumbnail_file: Cid,           // CID to an image file that is to be used as the thumbnail
	pub thumbnail_info: ThumbnailInfo, // thumbnail metadata
}

/**
 * Contains metadata of any location based content
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct LocationInfo {
	pub thumbnail_file: Cid,           // CID to an image file that is to be used as the thumbnail
	pub thumbnail_info: ThumbnailInfo, // thumbnail metadata
}
