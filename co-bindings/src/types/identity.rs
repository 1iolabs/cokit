// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_sdk::{Identity, PrivateIdentityBox};

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(opaque))]
#[derive(Debug, Clone)]
pub struct CoPrivateIdentity {
	pub(crate) identity: PrivateIdentityBox,
}
#[cfg_attr(feature = "uniffi", uniffi::export)]
impl CoPrivateIdentity {
	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(sync))]
	pub fn identity(&self) -> String {
		self.identity.identity().to_owned()
	}
}
impl From<PrivateIdentityBox> for CoPrivateIdentity {
	fn from(value: PrivateIdentityBox) -> Self {
		Self { identity: value }
	}
}
impl AsRef<PrivateIdentityBox> for CoPrivateIdentity {
	fn as_ref(&self) -> &PrivateIdentityBox {
		&self.identity
	}
}
