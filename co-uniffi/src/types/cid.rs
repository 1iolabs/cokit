use crate::CoError;
use cid::Cid;
use std::sync::Arc;

/// Binding for [`Cid`].
#[derive(Debug, Clone, uniffi::Record)]
#[uniffi(name = "Cid")]
pub struct CoCid {
	pub cid: Vec<u8>,
}
// #[uniffi::export]
// impl CoCid {
// 	#[uniffi::constructor]
// 	pub fn from_string(str: String) -> Result<CoCid, Arc<CoError>> {
// 		Ok(CoCid::from(Cid::try_from(str).map_err(CoError::new)?))
// 	}
// }
// uniffi::custom_type!(CoCid, Cid);
// impl UniffiCustomTypeConverter for Cid {
// 	type Builtin = CoCid;
// 	fn into_custom(val: Self::Builtin) -> uniffi::Result<Self>
// 	where
// 		Self: Sized,
// 	{
// 		Ok(Cid::try_from(val.cid)?)
// 	}
// 	fn from_custom(obj: Self) -> Self::Builtin {
// 		CoCid { cid: obj.to_bytes() }
// 	}
// }
impl TryFrom<CoCid> for Cid {
	type Error = cid::Error;

	fn try_from(value: CoCid) -> Result<Self, Self::Error> {
		value.cid.as_slice().try_into()
	}
}
impl TryFrom<&CoCid> for Cid {
	type Error = cid::Error;

	fn try_from(value: &CoCid) -> Result<Self, Self::Error> {
		value.cid.as_slice().try_into()
	}
}
impl From<Cid> for CoCid {
	fn from(value: Cid) -> Self {
		CoCid { cid: value.to_bytes() }
	}
}

#[uniffi::export]
pub fn cid_version(cid: &CoCid) -> Result<u64, Arc<CoError>> {
	Ok(Cid::try_from(cid.cid.as_slice()).map_err(CoError::new_arc)?.version().into())
}

#[uniffi::export]
pub fn cid_codec(cid: &CoCid) -> Result<u64, Arc<CoError>> {
	Ok(Cid::try_from(cid.cid.as_slice()).map_err(CoError::new_arc)?.codec())
}

#[uniffi::export]
pub fn cid_to_string(cid: &CoCid) -> Result<String, Arc<CoError>> {
	Ok(Cid::try_from(cid.cid.as_slice()).map_err(CoError::new_arc)?.to_string())
}
