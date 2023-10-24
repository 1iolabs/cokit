use crate::types::{codec::MultiCodec, storage::Storage};
use aead::stream::{Decryptor, Encryptor};
use chacha20poly1305::{
	aead::{Aead, AeadCore, KeyInit, OsRng},
	XChaCha20Poly1305,
};
use libipld::{
	multihash::{Code, MultihashDigest},
	Block, Cid, DefaultParams,
};
use serde::{Deserialize, Serialize};
use std::{
	borrow::BorrowMut,
	cell::RefCell,
	collections::BTreeMap,
	fmt::{Debug, Display},
};

struct Key {
	key: Vec<u8>,
}
impl Key {
	pub fn new(key: Vec<u8>) -> Self {
		Self { key }
	}

	pub fn key(&self) -> &Vec<u8> {
		&self.key
	}
}
impl Display for Key {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Key")
	}
}
impl Debug for Key {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let v = "*****".to_owned();
		f.debug_struct("Key").field("key", &v).finish()
	}
}

struct EncryptedStorage {
	key: Key,
	algorithm: Algorithm,
	next: Box<dyn Storage>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Algorithm {
	XChaCha20Poly1305([u8; 24]),
}
impl Algorithm {
	pub fn generate_XChaCha20Poly1305() -> Algorithm {
		Algorithm::XChaCha20Poly1305(XChaCha20Poly1305::generate_nonce(&mut OsRng).into())
	}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum HashAlgorithm {
	Argon2Id(),
}

impl Storage for EncryptedStorage {
	fn get(&self, cid: &Cid) -> Block<DefaultParams> {
		todo!()
	}

	fn set(&mut self, block: Block<DefaultParams>) {
		todo!()
	}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EncryptionVersion {
	V1 = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlock {
	pub version: EncryptionVersion,
	pub algorithm: Algorithm,
	pub keyslots: Vec<Keyslot>,
	pub cid: Cid,
	pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum KeyslotVersion {
	V1 = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Keyslot {
	pub version: KeyslotVersion,
	pub algorithm: Algorithm,
	// pub hash: HashAlgorithm,
	// pub hash_salt: Vec<u8>,
	/// Encrypted master key.
	/// The key encryption key is derived from the master key.
	pub key: Vec<u8>,
	pub salt: Vec<u8>,

	pub nonce: Vec<u8>,
}

struct Record {
	block: Block<DefaultParams>,
	pin: bool,
}

#[derive(Debug, Clone)]
pub struct AddOptions {
	pin: bool,
	codec: u64,
}
impl AddOptions {
	pub fn new(codec: u64) -> Self {
		Self { pin: Default::default(), codec }
	}
}
impl Default for AddOptions {
	fn default() -> Self {
		Self { pin: Default::default(), codec: MultiCodec::Raw as u64 }
	}
}

pub struct MemoryStorage {
	records: BTreeMap<Cid, Record>,
}

impl MemoryStorage {
	pub fn new() -> Self {
		Self { records: BTreeMap::new() }
	}

	pub fn set(&mut self, block: Block<DefaultParams>) {
		// let cid = Cid::new_v1(options.codec, Code::Blake3_256.digest(&data[..]));
		self.records.insert(block.cid().clone(), Record { pin: false, block });
	}

	pub fn get(&self, cid: &Cid) -> Option<Block<DefaultParams>> {
		self.records.get(cid).map(|r| r.block.clone())
	}

	pub fn pin(&mut self, cid: &Cid) -> bool {
		match self.records.get_mut(cid) {
			Some(r) => {
				r.pin = true;
				true
			},
			None => false,
		}
	}

	pub fn unpin(&mut self, cid: &Cid) -> bool {
		match self.records.get_mut(cid) {
			Some(r) => {
				r.pin = false;
				true
			},
			None => false,
		}
	}
}
