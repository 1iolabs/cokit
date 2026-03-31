// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
