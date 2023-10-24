/// MultiCodec
///
/// See: https://github.com/multiformats/multicodec/blob/master/table.csv
pub enum MultiCodec {
	Identity = 0x0,
	Raw = 0x55,
	DagCbor = 0x71,
}

impl TryFrom<u64> for MultiCodec {
	type Error = u64;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		match value {
			0x0 => Ok(MultiCodec::Identity),
			0x55 => Ok(MultiCodec::Raw),
			0x71 => Ok(MultiCodec::DagCbor),
			_ => Err(value),
		}
	}
}
