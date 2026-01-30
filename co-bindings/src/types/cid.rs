use crate::CoError;

/// Binding for [`Cid`].
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone)]
pub struct Cid {
	pub bytes: Vec<u8>,
}
impl Cid {
	pub fn from_string(string: String) -> Result<Self, CoError> {
		let cid = cid::Cid::try_from(string).map_err(CoError::new)?;
		Ok(Cid { bytes: cid.to_bytes() })
	}

	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(sync))]
	pub fn codec(&self) -> Result<u64, CoError> {
		Ok(self.cid()?.codec())
	}

	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(sync))]
	pub fn version(&self) -> Result<u64, CoError> {
		Ok(self.cid()?.version().into())
	}

	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(sync))]
	pub fn to_string(&self) -> Result<String, CoError> {
		Ok(self.cid()?.to_string())
	}

	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(ignore))]
	pub fn cid(&self) -> Result<cid::Cid, CoError> {
		cid::Cid::try_from(self.bytes.as_slice()).map_err(CoError::new)
	}
}
impl TryFrom<Cid> for cid::Cid {
	type Error = cid::Error;

	fn try_from(value: Cid) -> Result<Self, Self::Error> {
		value.bytes.as_slice().try_into()
	}
}
impl TryFrom<&Cid> for cid::Cid {
	type Error = cid::Error;

	fn try_from(value: &Cid) -> Result<Self, Self::Error> {
		value.bytes.as_slice().try_into()
	}
}
impl From<cid::Cid> for Cid {
	fn from(value: cid::Cid) -> Self {
		Cid { bytes: value.to_bytes() }
	}
}
impl From<&cid::Cid> for Cid {
	fn from(value: &cid::Cid) -> Self {
		Cid { bytes: value.to_bytes() }
	}
}

// #[cfg_attr(feature = "uniffi", uniffi::export)]
// pub fn cid_version(cid: &CoCid) -> Result<u64, CoError> {
// 	Ok(Cid::try_from(cid.cid.as_slice()).map_err(CoError::new)?.version().into())
// }

// #[cfg_attr(feature = "uniffi", uniffi::export)]
// pub fn cid_codec(cid: &CoCid) -> Result<u64, CoError> {
// 	Ok(Cid::try_from(cid.cid.as_slice()).map_err(CoError::new)?.codec())
// }

// #[cfg_attr(feature = "uniffi", uniffi::export)]
// pub fn cid_to_string(cid: &CoCid) -> Result<String, CoError> {
// 	Ok(Cid::try_from(cid.cid.as_slice()).map_err(CoError::new)?.to_string())
// }
