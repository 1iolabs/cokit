use co_sdk::{Identity, PrivateIdentityBox};

#[derive(Debug, uniffi::Object)]
pub struct CoPrivateIdentity {
	identity: PrivateIdentityBox,
}
#[uniffi::export]
impl CoPrivateIdentity {
	#[allow(clippy::inherent_to_string)]
	pub fn to_string(&self) -> String {
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
