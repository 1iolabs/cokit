use super::secret::Secret;
use aead::{generic_array::typenum::Unsigned, KeySizeUser, Payload};
use chacha20poly1305::{
	aead::{Aead, AeadCore, KeyInit, OsRng},
	Key, XChaCha20Poly1305,
};
use cid::Cid;
use co_primitives::{from_cbor, to_cbor, Block, KnownMultiCodec, MultiCodec, MultiCodecError, StoreParams};
use derive_more::From;
use multihash_codetable::{Code, MultihashDigest};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{cmp::min, collections::BTreeMap, fmt::Debug, mem::take};

/// blake3 KDF context for derive block keys from versioned co encryption key
///
/// [application] [commit timestamp] [purpose]", e.g., "example.com 2019-12-25 16:18:03 session tokens v1
pub const BLOCK_KEY_DERIVATION: &str = "co 2023-10-24T10:25:23Z block key derivation v1";
pub const BLOCK_DERIVATION: &str = "co 2023-10-26T14:31:38Z block derivation v1";
pub const BLOCK_MULTICODEC: u64 = KnownMultiCodec::CoEncryptedBlock as u64;

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
	InvalidArguments(#[source] anyhow::Error),

	#[error("Generic decoding error")]
	Decoding,

	#[error("Generic encoding error")]
	Encoding,

	#[error("Size is to large")]
	Size,
}
impl From<aead::Error> for AlgorithmError {
	fn from(_: aead::Error) -> Self {
		AlgorithmError::Cipher
	}
}
impl From<MultiCodecError> for AlgorithmError {
	fn from(value: MultiCodecError) -> Self {
		AlgorithmError::InvalidArguments(value.into())
	}
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
#[derive(Default)]
pub enum Algorithm {
	#[default]
	XChaCha20Poly1305 = 1,
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

	/// Cipher algorithm tag size in bytes.
	pub fn tag_size(&self) -> usize {
		match self {
			Algorithm::XChaCha20Poly1305 => <XChaCha20Poly1305 as AeadCore>::TagSize::USIZE,
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
			return Err(AlgorithmError::InvalidArguments(anyhow::anyhow!("nonce size")));
		}
		if self.key_size() != secret.divulge().len() {
			return Err(AlgorithmError::InvalidArguments(anyhow::anyhow!("key size")));
		}

		// encrypt
		match self {
			Algorithm::XChaCha20Poly1305 => {
				let cipher = XChaCha20Poly1305::new(Key::from_slice(secret.divulge()));
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
			return Err(AlgorithmError::InvalidArguments(anyhow::anyhow!("nonce size")));
		}
		if self.key_size() != secret.divulge().len() {
			return Err(AlgorithmError::InvalidArguments(anyhow::anyhow!("key size")));
		}

		// decrypt
		match self {
			Algorithm::XChaCha20Poly1305 => {
				let cipher = XChaCha20Poly1305::new(Key::from_slice(secret.divulge()));
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

/// Encrypted Block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlock {
	/// Encryption header for payload.
	#[serde(rename = "h")]
	pub header: Header,

	/// Encrypted [`BlockPayload`].
	#[serde(rename = "d")]
	pub payload: EncryptedData,
}
impl EncryptedBlock {
	/// Encrypt block (automatically generate block secret).
	pub fn encrypt(
		algorithm: Algorithm,
		secret: &Secret,
		block: impl Into<BlockPayload>,
	) -> Result<EncryptedBlock, AlgorithmError> {
		let block_secret = algorithm.generate_serect();
		Self::encrypt_with_block_secret(algorithm, secret, &block_secret, block)
	}

	/// Encrypt block with custom block secret.
	pub fn encrypt_with_block_secret(
		algorithm: Algorithm,
		secret: &Secret,
		block_secret: &Secret,
		block: impl Into<BlockPayload>,
	) -> Result<EncryptedBlock, AlgorithmError> {
		let block: BlockPayload = block.into();

		// derive data key
		let data_secret = block_secret.derive_serect(BLOCK_DERIVATION);

		// header
		let key_slot = KeySlot::new(algorithm, secret, block_secret)?;
		let header = Header::new(algorithm, vec![key_slot]);

		// result
		let aad = header.aad();

		// data
		let data = block.to_bytes().map_err(|_e| AlgorithmError::Encoding)?;

		// encrypt
		Ok(Self {
			payload: header
				.algorithm
				.encrypt(&data_secret, &header.nonce, data.as_slice(), aad.as_slice())?
				.into(),
			header,
		})
	}

	/// Get decrypted block.
	pub fn block(&self, secret: &Secret) -> Result<BlockPayload, AlgorithmError> {
		let block_secret = self
			.header
			.block_secret(secret)
			.ok_or(AlgorithmError::InvalidArguments(anyhow::anyhow!("key")))?;
		let aad = self.header.aad();
		let data = self
			.payload
			.inline()
			.ok_or(AlgorithmError::InvalidArguments(anyhow::anyhow!("Expected inline data")))?;
		let data_plain = self.decrypt_data(&block_secret, data, &aad)?;
		from_cbor(&data_plain).map_err(|err| AlgorithmError::InvalidArguments(err.into()))
	}

	fn decrypt_data(&self, block_secret: &Secret, data: &[u8], aad: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
		let data_secret = block_secret.derive_serect(BLOCK_DERIVATION);
		let data = self.header.algorithm.decrypt(&data_secret, &self.header.nonce, data, aad)?;
		Ok(data)
	}

	/// Test is encrypted block is valid.
	pub fn is_valid(&self) -> bool {
		self.header.is_valid()
	}
}
impl<S> TryInto<Block<S>> for EncryptedBlock
where
	S: StoreParams,
{
	type Error = AlgorithmError;

	/// Convert to encrypted Block.
	fn try_into(self) -> Result<Block<S>, Self::Error> {
		let encrypted_data = to_cbor(&self).map_err(|_| AlgorithmError::Encoding)?;
		let mh = Code::Blake3_256.digest(&encrypted_data);
		let cid = Cid::new_v1(KnownMultiCodec::CoEncryptedBlock.into(), mh);
		Ok(Block::new_unchecked(cid, encrypted_data))
	}
}
impl<S> TryFrom<Block<S>> for EncryptedBlock
where
	S: StoreParams,
{
	type Error = AlgorithmError;

	/// Convert from encrypted Block.
	fn try_from(value: Block<S>) -> Result<Self, Self::Error> {
		// validate
		MultiCodec::with_codec(KnownMultiCodec::CoEncryptedBlock, value.cid())?;

		// decode
		let block: EncryptedBlock = from_cbor(value.data()).map_err(|_| AlgorithmError::Decoding)?;

		// validate
		if !block.is_valid() {
			return Err(AlgorithmError::Decoding);
		}

		// result
		Ok(block)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(untagged)]
pub enum EncryptedData {
	/// Inline cipher text.
	#[from]
	#[serde(with = "serde_bytes")]
	Inline(Vec<CipherU8>),

	/// Referenced cipher text.
	/// Expected to be [`KnownMultiCodec::Raw`].
	#[from]
	Block(Vec<Cid>),
}
impl EncryptedData {
	pub fn inline(&self) -> Option<&[u8]> {
		match self {
			Self::Inline(data) => Some(data),
			_ => None,
		}
	}

	pub fn blocks(&self) -> Option<&[Cid]> {
		match self {
			Self::Block(data) => Some(data),
			_ => None,
		}
	}

	/// Fit [`EncryptedData`] into blocks.
	///
	/// If the [`EncryptedData`] doesn't fit into one block it will we splitted accordingly.
	/// This method should return at max 3 blocks.
	///
	/// # Returns
	/// The extra blocks or an empty Vec if it fits inline.
	pub fn fit_into_blocks<P: StoreParams>(&mut self, inline_offset: Option<usize>) -> Vec<Block<P>> {
		let mut data = match self {
			Self::Inline(data) => {
				if P::MAX_BLOCK_SIZE >= data.len() + inline_offset.unwrap_or(0) {
					return vec![];
				} else {
					take(data)
				}
			},
			Self::Block(_) => {
				return vec![];
			},
		};
		let mut extra_blocks = Vec::new();
		while !data.is_empty() {
			let rest = data.split_off(min(data.len(), P::MAX_BLOCK_SIZE));
			extra_blocks.push(Block::new_data(KnownMultiCodec::Raw, data));
			data = rest;
		}
		*self = Self::Block(extra_blocks.iter().map(|block| *block.cid()).collect());
		extra_blocks
	}

	/// Try to inline using blocks. Returns Err if not possible because blocks are missing.
	pub fn try_inline_blocks(&mut self, blocks: impl IntoIterator<Item = (Cid, Vec<u8>)>) -> Result<(), ()> {
		match self {
			Self::Inline(_) => Ok(()),
			Self::Block(cids) => {
				let mut blocks: BTreeMap<Cid, Vec<u8>> = blocks.into_iter().collect();
				if !cids.iter().all(|cid| blocks.contains_key(cid)) {
					return Err(());
				}
				let mut inline = Vec::new();
				for cid in cids {
					if let Some(mut block) = blocks.remove(cid) {
						inline.append(&mut block);
					} else {
						return Err(());
					}
				}
				*self = Self::Inline(inline);
				Ok(())
			},
		}
	}
}

/// Combines reference mappings and data into one structure.
/// The max size of this `MAX_BLOCK_SIZE * 3`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPayload {
	/// The block [`Cid`].
	#[serde(rename = "c")]
	pub cid: Cid,

	/// Optionally maps block references from unencrypted Cid (key) to encrypted Cid (value).
	/// When a mapping exists it is assumed to contain all links that are still resolvable.
	#[serde(rename = "r", default, skip_serializing_if = "BTreeMap::is_empty")]
	pub references: BTreeMap<Cid, Cid>,

	/// The block data.
	#[serde(with = "serde_bytes", rename = "d")]
	pub data: Vec<u8>,
}
impl BlockPayload {
	/// Returns the cid.
	pub fn cid(&self) -> &Cid {
		&self.cid
	}

	// /// Returns the payload.
	// pub fn data(&self) -> &[u8] {
	// 	&self.data
	// }

	/// Create plain bytes which contains the [`BlockPayload`] as DAG-CBOR.
	pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
		Ok(to_cbor(self)?)
	}
}
impl<S> From<Block<S>> for BlockPayload
where
	S: StoreParams,
{
	fn from(value: Block<S>) -> Self {
		let (cid, data) = value.into_inner();
		Self { cid, data, references: Default::default() }
	}
}
impl<S> From<BlockPayload> for Block<S>
where
	S: StoreParams,
{
	fn from(value: BlockPayload) -> Self {
		Block::new_unchecked(value.cid, value.data)
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
	pub fn new(algorithm: Algorithm, key_slots: Vec<KeySlot>) -> Self {
		Self { version: EncryptionVersion::V1, algorithm, nonce: algorithm.generate_nonce(), key_slots }
	}

	/// Test if header is valid.
	pub fn is_valid(&self) -> bool {
		self.version == EncryptionVersion::V1
			&& self.nonce.len() == self.algorithm.nonce_size()
			&& self.key_slots.iter().all(KeySlot::is_valid)
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
		result
	}

	/// Get block secret fot given secret.
	pub fn block_secret(&self, secret: &Secret) -> Option<Secret> {
		self.key_slots
			.iter()
			.map(|key_slot| key_slot.block_secret(secret))
			.filter_map(|r| r.ok())
			.next()
	}

	/// Calulate CBOR header encoded size with single key slot.
	///
	/// XChaCha20Poly1305: 153
	pub fn encoded_size(algorithm: Algorithm) -> usize {
		let field_size = 1;
		let cbor_size = 1;
		cbor_size
			// version
			+ 1 + field_size + cbor_size
			// algorithm
			+ 1 + field_size + cbor_size
			// key_slots
			+ KeySlot::encoded_size(algorithm) + field_size + cbor_size + cbor_size
			// nonce
			+ algorithm.nonce_size() + field_size + cbor_size + cbor_size + cbor_size
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
	/// Calulate CBOR encoded size.
	///
	/// XChaCha20Poly1305: 116
	pub fn encoded_size(algorithm: Algorithm) -> usize {
		let tag_size = algorithm.tag_size();
		let field_size = 1;
		let cbor_size = 1;
		cbor_size
			// version
			+ 1 + field_size + cbor_size
			// algorithm
			+ 1 + field_size + cbor_size
			// key
			+ algorithm.key_size() + field_size + tag_size + cbor_size + cbor_size + cbor_size
			// salt
			+ algorithm.nonce_size() + field_size + cbor_size + cbor_size + cbor_size
			// nonce
			+ algorithm.nonce_size() + field_size + cbor_size + cbor_size + cbor_size
	}

	/// Create new key slot using the CO Key (serect) and a generated block secret (may reused).
	pub fn new(algorithm: Algorithm, secret: &Secret, block_secret: &Secret) -> Result<Self, AlgorithmError> {
		let salt = algorithm.generate_nonce(); // TODO: needs specific size?
		let secret_derived = secret.derive_serect_with_salt(BLOCK_KEY_DERIVATION, &salt);
		let nonce = algorithm.generate_nonce();
		let block_secret_encrypted = algorithm.encrypt(&secret_derived, &nonce, block_secret.divulge(), b"")?;
		Ok(Self { version: KeySlotVersion::V1, algorithm, key: block_secret_encrypted, nonce, salt })
	}

	/// Test if is keyslot is valid.
	pub fn is_valid(&self) -> bool {
		self.version == KeySlotVersion::V1
			&& self.key.len() == self.algorithm.key_size() + self.algorithm.tag_size()
			&& self.nonce.len() == self.algorithm.nonce_size()
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
	use super::{Algorithm, EncryptedBlock, Header, KeySlot};
	use crate::crypto::{block::EncryptedData, secret::Secret};
	use cid::Cid;
	use co_primitives::{from_cbor, to_cbor, Block, BlockSerializer, DefaultParams, KnownMultiCodec, StoreParams};
	use std::iter::repeat_n;

	#[test]
	fn algorithm_key_size() {
		assert_eq!(Algorithm::XChaCha20Poly1305.key_size(), 32);
	}

	#[test]
	fn algorithm_nonce_size() {
		assert_eq!(Algorithm::XChaCha20Poly1305.nonce_size(), 24);
	}

	#[test]
	fn is_valid() {
		let secret = Secret::new(repeat_n(0u8, Algorithm::default().key_size()).collect());
		let block_secret = Secret::new(repeat_n(1u8, Algorithm::default().key_size()).collect());
		let key_slot = KeySlot::new(Algorithm::default(), &secret, &block_secret).unwrap();
		let header = Header::new(Algorithm::default(), vec![key_slot]);
		assert!(header.is_valid());
	}

	#[test]
	fn serialize_header() {
		let secret = Secret::new(repeat_n(0u8, Algorithm::default().key_size()).collect());
		let block_secret = Secret::new(repeat_n(1u8, Algorithm::default().key_size()).collect());
		let key_slot = KeySlot::new(Algorithm::default(), &secret, &block_secret).unwrap();
		let header = Header::new(Algorithm::default(), vec![key_slot]);

		// serialize header
		let bytes = to_cbor(&header).unwrap();
		// println!("{:?}", header);
		// let raw_bytes = Into::<Vec<u8>>::into(header.clone());
		// println!("raw_bytes: {}", raw_bytes.len()); // 129
		// println!("bytes: {}", bytes.len()); // 153 (153 - 129 = 24)
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
		assert_eq!(bytes.len(), 153);

		// deserialize
		let header_deserialized: Header = from_cbor(bytes.as_slice()).unwrap();
		assert_eq!(header_deserialized, header);
		assert!(header.is_valid());
	}

	#[test]
	fn key_slot_encoded_size() {
		let secret = Secret::new(repeat_n(0u8, Algorithm::default().key_size()).collect());
		let block_secret = Secret::new(repeat_n(1u8, Algorithm::default().key_size()).collect());
		let key_slot = KeySlot::new(Algorithm::default(), &secret, &block_secret).unwrap();

		// serialize header
		let bytes = to_cbor(&key_slot).unwrap();
		//hexdump::hexdump(bytes.as_slice());
		assert_eq!(bytes.len(), KeySlot::encoded_size(Algorithm::default()));
	}

	#[test]
	fn header_encoded_size() {
		let secret = Secret::new(repeat_n(0u8, Algorithm::default().key_size()).collect());
		let block_secret = Secret::new(repeat_n(1u8, Algorithm::default().key_size()).collect());
		let key_slot = KeySlot::new(Algorithm::default(), &secret, &block_secret).unwrap();
		let header = Header::new(Algorithm::default(), vec![key_slot]);

		// serialize header
		let bytes = to_cbor(&header).unwrap();
		//hexdump::hexdump(bytes.as_slice());
		assert_eq!(bytes.len(), Header::encoded_size(Algorithm::default()));
	}

	#[test]
	fn encrypt_block_roundtrip() {
		let secret = Secret::new(repeat_n(0u8, Algorithm::default().key_size()).collect());
		let block = BlockSerializer::default().serialize(&"Hello World!").unwrap();

		//println!("cid: ({}): {}", block.cid().to_bytes().len(), block.cid()); // 36
		//println!("data: ({}): {:?}", block.data().len(), block.data()); // 13

		// encrypt
		let encrypted_block = EncryptedBlock::encrypt(Algorithm::default(), &secret, block.clone()).unwrap();
		assert_ne!(encrypted_block.payload.inline().unwrap(), block.data());
		//println!("cid: ({}): {:?}", encrypted_block.cid.len(), encrypted_block.cid); // 52 = 36 + 16
		// println!("data: ({}): {:?}", encrypted_block.payload.inline().unwrap().len(), encrypted_block.payload); // 76

		// serialize
		let encrypted_block_bytes = to_cbor(&encrypted_block).unwrap();
		// cbor (7), header (153), payload+tag (76)
		assert_eq!(encrypted_block_bytes.len(), 236);
		//println!("length: {}", encrypted_block_bytes.len());
		//hexdump::hexdump(&encrypted_block_bytes);

		// deserialize
		let encrypted_block_deserialized: EncryptedBlock = from_cbor(&encrypted_block_bytes).unwrap();

		// decrypt
		let decrypted_block = encrypted_block_deserialized.block(&secret).unwrap();
		assert_eq!(decrypted_block.cid(), block.cid());
		assert_eq!(&decrypted_block.data, block.data());
	}

	#[test]
	fn test_fit_to_blocks() {
		let secret = Secret::new(repeat_n(0u8, Algorithm::default().key_size()).collect());
		let data: Vec<u8> = repeat_n(0u8, DefaultParams::MAX_BLOCK_SIZE).collect();
		let block = Block::<DefaultParams>::new_data(KnownMultiCodec::Raw, data);

		//println!("cid: ({}): {}", block.cid().to_bytes().len(), block.cid()); // 36
		//println!("data: ({}): {:?}", block.data().len(), block.data()); // 13

		// encrypt
		let mut encrypted_block = EncryptedBlock::encrypt(Algorithm::default(), &secret, block.clone()).unwrap();

		// split
		let encrypted_extra_blocks = encrypted_block
			.payload
			.fit_into_blocks::<DefaultParams>(Some(Header::encoded_size(Algorithm::default())));
		assert!(match &encrypted_block.payload {
			EncryptedData::Block(blocks) =>
				blocks == &encrypted_extra_blocks.iter().map(|b| *b.cid()).collect::<Vec<Cid>>(),
			_ => false,
		});

		// inline
		encrypted_block
			.payload
			.try_inline_blocks(encrypted_extra_blocks.into_iter().map(|v| v.into_inner()))
			.unwrap();

		// decrypt
		let decrypted_block = encrypted_block.block(&secret).unwrap();
		assert_eq!(decrypted_block.cid(), block.cid());
		assert_eq!(&decrypted_block.data, block.data());
	}
}
