use aead::rand_core::RngCore;
use blake3::derive_key;
use chacha20poly1305::aead::OsRng;
use std::fmt::{Debug, Display};
use zeroize::Zeroize;

/// Stores a secrent and ensures it will only be showen using the divulge method.
/// Display and debug traits are implemented and will only show masked keys.
#[derive(Clone)]
pub struct Secret {
	secret: Vec<u8>,
}
impl Secret {
	/// Create secret from vec.
	pub fn new(key: Vec<u8>) -> Self {
		Self { secret: key }
	}

	/// Generate random secret of given size.
	pub fn generate(size: usize) -> Self {
		let mut secret: Vec<u8> = Vec::with_capacity(size);
		secret.resize(size, 0);
		OsRng.fill_bytes(secret.as_mut_slice());
		Self::new(secret)
	}

	/// Derive secret.
	pub fn derive_serect(&self, context: &str) -> Secret {
		Secret::new(derive_key(context, self.divulge().as_slice()).to_vec())
	}

	/// Derive secret with salt.
	pub fn derive_serect_with_salt(&self, context: &str, salt: &Vec<u8>) -> Secret {
		// append the salt
		let salted_secret = {
			let mut with_salt = self.divulge().clone();
			with_salt.extend_from_slice(salt.as_slice());
			Secret::new(with_salt)
		};

		// derive
		salted_secret.derive_serect(context)
	}

	/// Divulge the secret.
	pub fn divulge(&self) -> &Vec<u8> {
		&self.secret
	}
}
impl Display for Secret {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Secret")
	}
}
impl Debug for Secret {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let v = "*****".to_owned(); //?
		f.debug_struct("Secret").field("secret", &v).finish()
	}
}
impl Drop for Secret {
	fn drop(&mut self) {
		self.secret.zeroize()
	}
}
