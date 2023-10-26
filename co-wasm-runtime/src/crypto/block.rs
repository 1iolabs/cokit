use super::Secret;
use aead::{generic_array::typenum::Unsigned, KeySizeUser, Payload};
use chacha20poly1305::{
	aead::{Aead, AeadCore, KeyInit, OsRng},
	Key, XChaCha20Poly1305,
};
use libipld::{store::StoreParams, Block, Cid};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{fmt::Debug, marker::PhantomData};

/// blake3 KDF context for derive block keys from versioned co encryption key
///
/// [application] [commit timestamp] [purpose]", e.g., "example.com 2019-12-25 16:18:03 session tokens v1
pub const BLOCK_KEY_DERIVATION: &str = "co 2023-10-24T10:25:23Z block key derivation v1";
pub const BLOCK_DERIVATION: &str = "co 2023-10-26T14:31:38Z block derivation v1";

/// Nonce.
pub type Nonce = Vec<u8>;
/// Salt.
pub type Salt = Vec<u8>;
/// Cipher octet (encrypted).
pub type CipherU8 = u8;

#[derive(Debug, thiserror::Error)]
pub enum AlgorithmError {
	#[error("Generic Cipher Error")]
	Cipher,

	#[error("Invalid arguments specified")]
	InvalidArguments,

	#[error("Generic decoding error")]
	Decoding,
}
impl From<aead::Error> for AlgorithmError {
	fn from(_: aead::Error) -> Self {
		AlgorithmError::Cipher
	}
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum Algorithm {
	XChaCha20Poly1305 = 1,
}
impl Default for Algorithm {
	fn default() -> Self {
		Self::XChaCha20Poly1305
	}
}
impl Algorithm {
	/// Cipher algorithm key size in bytes.
	pub fn key_size(&self) -> usize {
		match self {
			Algorithm::XChaCha20Poly1305 => XChaCha20Poly1305::key_size(),
		}
	}

	/// Cipher algorithm nonce size in bytes.
	pub fn nonce_size(&self) -> usize {
		match self {
			Algorithm::XChaCha20Poly1305 => <XChaCha20Poly1305 as AeadCore>::NonceSize::USIZE,
		}
	}

	/// Generate a random secret key suitable for the cipher algorithm.
	pub fn generate_serect(&self) -> Secret {
		match self {
			Algorithm::XChaCha20Poly1305 => Secret::new(XChaCha20Poly1305::generate_key(&mut OsRng).to_vec()),
		}
	}

	/// Generate a random nonce suitable for the cipher algorithm.
	pub fn generate_nonce(&self) -> Nonce {
		match self {
			Algorithm::XChaCha20Poly1305 => XChaCha20Poly1305::generate_nonce(&mut OsRng).to_vec(),
		}
	}

	/// Encrypt single buffer of data.
	pub fn encrypt(
		&self,
		secret: &Secret,
		nonce: &Nonce,
		plaintext: &[u8],
		aad: &[u8],
	) -> Result<Vec<u8>, AlgorithmError> {
		// validate
		if self.nonce_size() != nonce.len() {
			return Err(AlgorithmError::InvalidArguments)
		}
		if self.key_size() != secret.divulge().len() {
			return Err(AlgorithmError::InvalidArguments)
		}

		// encrypt
		match self {
			Algorithm::XChaCha20Poly1305 => {
				let cipher = XChaCha20Poly1305::new(Key::from_slice(secret.divulge().as_slice()));
				let payload = Payload { msg: plaintext, aad };
				cipher
					.encrypt(aead::Nonce::<XChaCha20Poly1305>::from_slice(nonce.as_slice()), payload)
					.map_err(|e| e.into())
			},
		}
	}

	/// Decrypt single buffer of data.
	pub fn decrypt(
		&self,
		secret: &Secret,
		nonce: &Nonce,
		ciphertext: &[CipherU8],
		aad: &[u8],
	) -> Result<Vec<u8>, AlgorithmError> {
		// validate
		if self.nonce_size() != nonce.len() {
			return Err(AlgorithmError::InvalidArguments)
		}
		if self.key_size() != secret.divulge().len() {
			return Err(AlgorithmError::InvalidArguments)
		}

		// decrypt
		match self {
			Algorithm::XChaCha20Poly1305 => {
				let cipher = XChaCha20Poly1305::new(Key::from_slice(secret.divulge().as_slice()));
				let payload = Payload { msg: ciphertext, aad };
				cipher
					.decrypt(aead::Nonce::<XChaCha20Poly1305>::from_slice(nonce.as_slice()), payload)
					.map_err(|e| e.into())
			},
		}
	}
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum EncryptionVersion {
	V1 = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlock<S> {
	#[serde(skip)]
	_marker: PhantomData<S>,

	/// Encryption header for payload.
	#[serde(rename = "h")]
	header: Header,

	// Encrypted binary data.
	#[serde(rename = "c", with = "serde_bytes")]
	cid: Vec<CipherU8>,

	/// Encrypted binary data.
	#[serde(rename = "d", with = "serde_bytes")]
	data: Vec<CipherU8>,
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// struct EncryptedBlockRepr {
// 	/// Encrypted binary data.
// 	cid: Cid,

// 	/// Binary data.
// 	data: Vec<u8>,
// }
// impl<S> From<Block<S>> for EncryptedBlockRepr
// where
// 	S: StoreParams,
// {
// 	fn from(value: Block<S>) -> Self {
// 		let (cid, data) = value.into_inner();
// 		Self { cid, data }
// 	}
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(remote = "Block")]
// struct BlockDef<S>
// where
// 	S: StoreParams,
// {
// 	#[serde(skip_serializing)]
// 	_marker: PhantomData<S>,
// 	/// Content identifier.
// 	#[serde(rename = "c", getter = "Block::cid")]
// 	cid: Cid,
// 	/// Binary data.
// 	#[serde(rename = "d", getter = "Block::data", with = "serde_bytes")]
// 	data: Vec<u8>,
// }
// impl<S> Into<Block<S>> for BlockDef<S>
// where
// 	S: StoreParams,
// {
// 	fn into(self) -> Block<S> {
// 		Block::new_unchecked(self.cid, self.data)
// 	}
// }

impl<S> EncryptedBlock<S>
where
	S: StoreParams,
{
	pub fn encrypt(secret: &Secret, block: Block<S>) -> Result<EncryptedBlock<S>, AlgorithmError> {
		let block_secret = Algorithm::default().generate_serect();
		Self::encrypt_with_block_secret(secret, &block_secret, block)
	}

	pub fn encrypt_with_block_secret(
		secret: &Secret,
		block_secret: &Secret,
		block: Block<S>,
	) -> Result<EncryptedBlock<S>, AlgorithmError> {
		// dervice data key
		let data_secret = block_secret.derive_serect(BLOCK_DERIVATION);

		// header
		let key_slot = KeySlot::new(secret, block_secret)?;
		let header = Header::new(vec![key_slot]);

		// result
		let (cid, data) = block.into_inner();
		let aad = header.aad();
		Ok(Self {
			_marker: Default::default(),
			cid: header
				.algorithm
				.encrypt(&block_secret, &header.nonce, cid.to_bytes().as_slice(), aad.as_slice())?,
			data: header
				.algorithm
				.encrypt(&data_secret, &header.nonce, data.as_slice(), aad.as_slice())?,
			header,
		})
	}

	pub fn block(&self, secret: &Secret) -> Result<Block<S>, AlgorithmError> {
		let block_secret = self.header.block_secret(secret).ok_or(AlgorithmError::InvalidArguments)?;
		let aad = self.header.aad();
		Ok(Block::new_unchecked(self.decrypt_cid(&block_secret, &aad)?, self.decrypt_data(&block_secret, &aad)?))
	}

	pub fn cid(&self, secret: &Secret) -> Result<Cid, AlgorithmError> {
		let block_secret = self.header.block_secret(secret).ok_or(AlgorithmError::InvalidArguments)?;
		let aad = self.header.aad();
		self.decrypt_cid(&block_secret, &aad)
	}

	pub fn data(&self, secret: &Secret) -> Result<Vec<u8>, AlgorithmError> {
		let block_secret = self.header.block_secret(secret).ok_or(AlgorithmError::InvalidArguments)?;
		let aad = self.header.aad();
		self.decrypt_data(&block_secret, &aad)
	}

	fn decrypt_cid(&self, block_secret: &Secret, aad: &[u8]) -> Result<Cid, AlgorithmError> {
		let cid = self
			.header
			.algorithm
			.decrypt(block_secret, &self.header.nonce, &self.cid, &aad)?;
		Ok(Cid::try_from(cid).map_err(|_| AlgorithmError::Decoding)?)
	}

	fn decrypt_data(&self, block_secret: &Secret, aad: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
		let data_secret = block_secret.derive_serect(BLOCK_DERIVATION);
		let data = self
			.header
			.algorithm
			.decrypt(&data_secret, &self.header.nonce, &self.data, &aad)?;
		Ok(data)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Header {
	/// Version.
	#[serde(rename = "v")]
	pub version: EncryptionVersion,

	/// Encryption algorithm for payload.
	#[serde(rename = "a")]
	pub algorithm: Algorithm,

	/// Keyslots for payload.
	#[serde(rename = "k")]
	pub key_slots: Vec<KeySlot>,

	/// Encryption nonce (iv) used for payload.
	#[serde(rename = "n", with = "serde_bytes")]
	pub nonce: Nonce,
}
impl Header {
	pub fn new(key_slots: Vec<KeySlot>) -> Self {
		let algorithm = Algorithm::default();
		Self { version: EncryptionVersion::V1, algorithm, nonce: algorithm.generate_nonce(), key_slots }
	}

	/// Get AAD bytes for this header.
	pub fn aad(&self) -> Vec<u8> {
		let mut result = Vec::with_capacity(1 + 1 + self.nonce.len());
		result.extend([self.version as u8, self.algorithm as u8].iter());
		result.extend(self.nonce.iter());
		// let i = ipld!({
		// 	"v": self.version as usize,
		// 	"a": self.algorithm as usize,
		// 	"n": self.nonce.clone(),
		// });
		// DagCborCodec.encode(&i).unwrap().to_vec()
		return result
	}

	/// Get block secret fot given secret.
	pub fn block_secret(&self, secret: &Secret) -> Option<Secret> {
		self.key_slots
			.iter()
			.map(|key_slot| key_slot.block_secret(secret))
			.filter_map(|r| r.ok())
			.next()
	}
}
// impl Into<Vec<u8>> for Header {
// 	fn into(self) -> Vec<u8> {
// 		let mut result = Vec::with_capacity(1 + 1 + (1 + 1 + 32 + 24 + 24) + 24);
// 		result.extend([self.version as u8, self.algorithm as u8].iter());
// 		result.extend([self.key_slots.len() as u8].iter());
// 		result.extend(self.key_slots.into_iter().map(|k| Into::<Vec<u8>>::into(k)).flatten());
// 		result.extend([self.nonce.len() as u8].iter());
// 		result.extend(self.nonce.into_iter());
// 		result
// 	}
// }

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum KeySlotVersion {
	/// Key slot version 1.
	///
	/// Key Derivation: blake3
	V1 = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeySlot {
	/// Key slot version.
	#[serde(rename = "v")]
	pub version: KeySlotVersion,

	/// The algorithm used to encrypt the key.
	#[serde(rename = "a")]
	pub algorithm: Algorithm,

	/// Encrypted master key.
	/// The key encryption key is derived from the master key.
	/// Key Derivation Hash: blake3
	#[serde(rename = "k", with = "serde_bytes")]
	pub key: Vec<CipherU8>,

	/// Key derivation salt.
	#[serde(rename = "s", with = "serde_bytes")]
	pub salt: Salt,

	/// Key enctyion nonce.
	#[serde(rename = "n", with = "serde_bytes")]
	pub nonce: Nonce,
}
impl KeySlot {
	/// Create new key slot using the CO Key (serect) and a generated block secret (may reused).
	pub fn new(secret: &Secret, block_secret: &Secret) -> Result<Self, AlgorithmError> {
		let algorithm = Algorithm::default();
		let salt = algorithm.generate_nonce(); // TODO: needs specific size?
		let secret_derived = secret.derive_serect_with_salt(BLOCK_KEY_DERIVATION, &salt);
		let nonce = algorithm.generate_nonce();
		let block_secret_encrypted =
			algorithm.encrypt(&secret_derived, &nonce, block_secret.divulge().as_slice(), b"")?;
		Ok(Self { version: KeySlotVersion::V1, algorithm, key: block_secret_encrypted, nonce, salt })
	}

	/// Get block secret from key slot.
	pub fn block_secret(&self, secret: &Secret) -> Result<Secret, AlgorithmError> {
		let secret_derived = secret.derive_serect_with_salt(BLOCK_KEY_DERIVATION, &self.salt);
		let block_secret = self.algorithm.decrypt(&secret_derived, &self.nonce, self.key.as_slice(), b"")?;
		Ok(Secret::new(block_secret))
	}
}
// impl Into<Vec<u8>> for KeySlot {
// 	fn into(self) -> Vec<u8> {
// 		let mut result = Vec::with_capacity(1 + 1 + (1 + 32) + (1 + 24) + (1 + 24));
// 		result.extend([self.version as u8, self.algorithm as u8].iter());
// 		result.extend([self.key.len() as u8].iter());
// 		result.extend(self.key.into_iter());
// 		result.extend([self.salt.len() as u8].iter());
// 		result.extend(self.salt.into_iter());
// 		result.extend([self.nonce.len() as u8].iter());
// 		result.extend(self.nonce.into_iter());
// 		result
// 	}
// }

#[cfg(test)]
mod tests {
	use libipld::{cbor::DagCborCodec, multihash::Code, Block, DefaultParams};

	use super::{Algorithm, EncryptedBlock, Header, KeySlot};
	use crate::crypto::secret::Secret;
	use std::iter::repeat;

	#[test]
	fn algorithm_key_size() {
		assert_eq!(32, Algorithm::XChaCha20Poly1305.key_size());
	}

	#[test]
	fn algorithm_nonce_size() {
		assert_eq!(24, Algorithm::XChaCha20Poly1305.nonce_size());
	}

	#[test]
	fn serialize_header() {
		let secret = Secret::new(repeat(0u8).take(Algorithm::default().key_size()).collect());
		let block_secret = Secret::new(repeat(1u8).take(Algorithm::default().key_size()).collect());
		let key_slot = KeySlot::new(&secret, &block_secret).unwrap();
		let header = Header::new(vec![key_slot]);

		// serialize
		let bytes = serde_ipld_dagcbor::to_vec(&header).unwrap();
		// println!("{:?}", header);
		// let raw_bytes = Into::<Vec<u8>>::into(header.clone());
		// println!("raw_bytes: {}", raw_bytes.len()); // 129
		// println!("bytes: {}", bytes.len()); // 153
		// hexdump::hexdump(Into::<Vec<u8>>::into(header.clone()).as_slice());
		// println!("key");
		// hexdump::hexdump(header.key_slots[0].key.as_slice());
		// println!("key salt");
		// hexdump::hexdump(header.key_slots[0].salt.as_slice());
		// println!("key nonce");
		// hexdump::hexdump(header.key_slots[0].nonce.as_slice());
		// println!("nonce");
		// hexdump::hexdump(header.nonce.as_slice());
		// println!("bytes");
		// hexdump::hexdump(bytes.as_slice());
		assert_eq!(153, bytes.len());

		// deserialize
		let header_deserialized: Header = serde_ipld_dagcbor::from_slice(bytes.as_slice()).unwrap();
		assert_eq!(header, header_deserialized);
	}

	#[test]
	fn encrypt_block_roundtrip() {
		let secret = Secret::new(repeat(0u8).take(Algorithm::default().key_size()).collect());
		let block = Block::<DefaultParams>::encode(DagCborCodec, Code::Blake3_256, "Hello World!").unwrap();

		println!("cid: ({}): {}", block.cid().to_bytes().len(), block.cid()); // 36
		println!("data: ({}): {:?}", block.data().len(), block.data()); // 13

		// encrypt
		let encrypted_block = EncryptedBlock::encrypt(&secret, block.clone()).unwrap();
		assert_ne!(block.data(), encrypted_block.data);
		println!("cid: ({}): {:?}", encrypted_block.cid.len(), encrypted_block.cid); // 52 = 36 + 16
		println!("data: ({}): {:?}", encrypted_block.data.len(), encrypted_block.data); // 29 = 13 + 16

		// serialize
		let encrypted_block_bytes = serde_ipld_dagcbor::to_vec(&encrypted_block).unwrap();
		assert_eq!(245, encrypted_block_bytes.len()); // header (153), cid + tag (52), data + tag (29)
		println!("length: {}", encrypted_block_bytes.len());
		hexdump::hexdump(&encrypted_block_bytes);

		// deserialize
		let encrypted_block_deserialized: EncryptedBlock<DefaultParams> =
			serde_ipld_dagcbor::from_slice(&encrypted_block_bytes).unwrap();

		// decrypt
		let decrypted_block = encrypted_block_deserialized.block(&secret).unwrap();
		assert_eq!(block.cid(), decrypted_block.cid());
		assert_eq!(block.data(), decrypted_block.data());
	}
}
