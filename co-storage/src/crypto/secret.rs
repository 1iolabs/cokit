use aead::rand_core::RngCore;
use blake3::derive_key;
use chacha20poly1305::aead::OsRng;
use std::fmt::Display;

/// Stores a secrent and ensures it will only be showen using the divulge method.
/// Display and debug traits are implemented and will only show masked keys.
#[derive(Clone, Debug)]
pub struct Secret(co_primitives::Secret);
impl Secret {
	/// Create secret from vec.
	pub fn new(secret: Vec<u8>) -> Self {
		Self(co_primitives::Secret::new(secret))
	}

	/// Generate random secret of given size.
	pub fn generate(size: usize) -> Self {
		let mut secret: Vec<u8> = vec![0; size];
		OsRng.fill_bytes(secret.as_mut_slice());
		Self::new(secret)
	}

	/// Derive secret.
	pub fn derive_serect(&self, context: &str) -> Secret {
		Secret::new(derive_key(context, self.divulge()).to_vec())
	}

	/// Derive secret with salt.
	pub fn derive_serect_with_salt(&self, context: &str, salt: &Vec<u8>) -> Secret {
		// append the salt
		let salted_secret = {
			let mut with_salt = self.divulge().to_vec();
			with_salt.extend_from_slice(salt.as_slice());
			Secret::new(with_salt)
		};

		// derive
		salted_secret.derive_serect(context)
	}

	/// Divulge the secret.
	pub fn divulge(&self) -> &[u8] {
		self.0.divulge()
	}
}
impl Display for Secret {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
impl From<Secret> for co_primitives::Secret {
	fn from(val: Secret) -> Self {
		val.0
	}
}
impl From<co_primitives::Secret> for Secret {
	fn from(value: co_primitives::Secret) -> Self {
		Self(value)
	}
}
