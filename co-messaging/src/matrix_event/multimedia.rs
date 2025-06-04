use cid::Cid;
use co_macros::co_data;
use co_primitives::CoCid;

/// Contains metadata of images
#[co_data]
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
pub struct LocationInfo {
	/// CID to an image file that is to be used as the thumbnail
	#[schemars(with = "CoCid")]
	pub thumbnail_file: Cid,
	/// Thumbnail metadata
	pub thumbnail_info: ThumbnailInfo,
}
