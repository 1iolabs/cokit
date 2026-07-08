// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use aes_kw::{cipher::KeyInit as AesKwKeyInit, KwAes256};
use anyhow::{anyhow, ensure};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chacha20poly1305::{
	aead::{Aead, Payload},
	KeyInit, XChaCha20Poly1305, XNonce,
};
use rand::RngCore;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

const ALG: &str = "ECDH-ES+A256KW";
const ENC: &str = "XC20P";
const TYP: &str = "application/didcomm-encrypted+json";
const CEK_LEN: usize = 32;
const WRAPPED_CEK_LEN: usize = 40;
const TAG_LEN: usize = 16;
const XC20P_NONCE_LEN: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AnoncryptEnvelope {
	pub protected: String,
	pub recipients: Vec<Recipient>,
	pub ciphertext: String,
	pub iv: String,
	pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Recipient {
	pub header: RecipientHeader,
	pub encrypted_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RecipientHeader {
	pub kid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ProtectedHeader {
	pub typ: String,
	pub alg: String,
	pub enc: String,
	pub epk: Epk,
	pub apv: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub skid: Option<serde_json::Value>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub apu: Option<serde_json::Value>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub crit: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Epk {
	pub kty: String,
	pub crv: String,
	pub x: String,
}

pub(super) fn seal(plaintext: &[u8], recipient_kid: &str, recipient_public_key: &[u8]) -> anyhow::Result<String> {
	let recipient_public_key = x25519_key(recipient_public_key)?;
	let ephemeral_secret = StaticSecret::random_from_rng(rand::rngs::OsRng);
	let ephemeral_public = PublicKey::from(&ephemeral_secret);
	let shared_secret = ephemeral_secret.diffie_hellman(&PublicKey::from(recipient_public_key));
	ensure!(shared_secret.was_contributory(), "X25519 shared secret was not contributory");

	let apv_bytes = party_v_info(&[recipient_kid.to_owned()]);
	let mut cek = [0u8; CEK_LEN];
	rand::rngs::OsRng.fill_bytes(&mut cek);

	let protected_header = ProtectedHeader {
		typ: TYP.to_owned(),
		alg: ALG.to_owned(),
		enc: ENC.to_owned(),
		epk: Epk { kty: "OKP".to_owned(), crv: "X25519".to_owned(), x: encode_b64(ephemeral_public.as_bytes()) },
		apv: encode_b64(apv_bytes),
		skid: None,
		apu: None,
		crit: None,
	};
	let protected = encode_json(&protected_header)?;
	let kek = concat_kdf(shared_secret.as_bytes(), ALG, None, Some(&apv_bytes));
	let wrapped_cek = wrap_cek(&kek, &cek)?;

	let mut nonce = [0u8; XC20P_NONCE_LEN];
	rand::rngs::OsRng.fill_bytes(&mut nonce);
	let cipher = XChaCha20Poly1305::new((&cek).into());
	let encrypted = cipher
		.encrypt(XNonce::from_slice(&nonce), Payload { msg: plaintext, aad: protected.as_bytes() })
		.map_err(|_| anyhow!("XC20P encryption failed"))?;
	ensure!(encrypted.len() >= TAG_LEN, "XC20P output shorter than authentication tag");
	let (ciphertext, tag) = encrypted.split_at(encrypted.len() - TAG_LEN);

	let envelope = AnoncryptEnvelope {
		protected,
		recipients: vec![Recipient {
			header: RecipientHeader { kid: recipient_kid.to_owned() },
			encrypted_key: encode_b64(wrapped_cek),
		}],
		ciphertext: encode_b64(ciphertext),
		iv: encode_b64(nonce),
		tag: encode_b64(tag),
	};
	Ok(serde_json::to_string(&envelope)?)
}

pub(super) fn open(incoming: &str, recipient_private_key: &[u8]) -> anyhow::Result<Vec<u8>> {
	let recipient_private_key = x25519_key(recipient_private_key)?;
	let envelope: AnoncryptEnvelope = serde_json::from_str(incoming)?;
	ensure!(!envelope.recipients.is_empty(), "anoncrypt JWE has no recipients");

	let protected: ProtectedHeader = decode_json(&envelope.protected)?;
	ensure!(protected.typ == TYP, "unexpected JWE typ: {}", protected.typ);
	ensure!(protected.alg == ALG, "unexpected JWE alg: {}", protected.alg);
	ensure!(protected.enc == ENC, "unexpected JWE enc: {}", protected.enc);
	ensure!(protected.epk.kty == "OKP", "unexpected epk kty: {}", protected.epk.kty);
	ensure!(protected.epk.crv == "X25519", "unexpected epk crv: {}", protected.epk.crv);
	ensure!(protected.skid.is_none(), "anoncrypt JWE must not contain skid");
	ensure!(protected.apu.is_none(), "anoncrypt JWE must not contain apu");
	ensure!(protected.crit.is_none(), "anoncrypt JWE must not contain crit");

	let recipient_kids = envelope
		.recipients
		.iter()
		.map(|recipient| recipient.header.kid.clone())
		.collect::<Vec<_>>();
	let expected_apv = party_v_info(&recipient_kids);
	let actual_apv = decode_b64(&protected.apv)?;
	ensure!(actual_apv == expected_apv, "apv does not match recipient kid list");

	let epk = PublicKey::from(x25519_key(&decode_b64(&protected.epk.x)?)?);
	let recipient_secret = StaticSecret::from(recipient_private_key);
	let shared_secret = recipient_secret.diffie_hellman(&epk);
	ensure!(shared_secret.was_contributory(), "X25519 shared secret was not contributory");
	let kek = concat_kdf(shared_secret.as_bytes(), ALG, None, Some(&actual_apv));

	let mut last_error = None;
	for recipient in envelope.recipients {
		let encrypted_key = match decode_b64(&recipient.encrypted_key) {
			Ok(encrypted_key) => encrypted_key,
			Err(err) => {
				last_error = Some(err);
				continue;
			},
		};
		match unwrap_cek(&kek, &encrypted_key) {
			Ok(cek) => {
				return decrypt_payload(&envelope.protected, &envelope.iv, &envelope.ciphertext, &envelope.tag, &cek)
			},
			Err(err) => last_error = Some(err),
		}
	}
	Err(last_error.unwrap_or_else(|| anyhow!("no recipient key could unwrap the CEK")))
}

pub(crate) fn recipient_kids(incoming: &str) -> anyhow::Result<Vec<String>> {
	let envelope: AnoncryptEnvelope = serde_json::from_str(incoming)?;
	ensure!(!envelope.recipients.is_empty(), "anoncrypt JWE has no recipients");

	let protected: ProtectedHeader = decode_json(&envelope.protected)?;
	ensure!(protected.typ == TYP, "unexpected JWE typ: {}", protected.typ);
	ensure!(protected.alg == ALG, "unexpected JWE alg: {}", protected.alg);
	ensure!(protected.enc == ENC, "unexpected JWE enc: {}", protected.enc);

	Ok(envelope.recipients.into_iter().map(|recipient| recipient.header.kid).collect())
}

fn decrypt_payload(
	protected: &str,
	iv: &str,
	ciphertext: &str,
	tag: &str,
	cek: &[u8; CEK_LEN],
) -> anyhow::Result<Vec<u8>> {
	let nonce = decode_b64(iv)?;
	ensure!(nonce.len() == XC20P_NONCE_LEN, "invalid XC20P nonce length");
	let ciphertext = decode_b64(ciphertext)?;
	let tag = decode_b64(tag)?;
	ensure!(tag.len() == TAG_LEN, "invalid XC20P tag length");

	let mut encrypted = ciphertext;
	encrypted.extend(tag);
	let cipher = XChaCha20Poly1305::new(cek.into());
	cipher
		.decrypt(XNonce::from_slice(&nonce), Payload { msg: &encrypted, aad: protected.as_bytes() })
		.map_err(|_| anyhow!("XC20P decryption failed"))
}

fn x25519_key(bytes: &[u8]) -> anyhow::Result<[u8; 32]> {
	bytes.try_into().map_err(|_| anyhow!("X25519 key must be 32 bytes"))
}

fn wrap_cek(kek: &[u8; CEK_LEN], cek: &[u8; CEK_LEN]) -> anyhow::Result<[u8; WRAPPED_CEK_LEN]> {
	let kw = KwAes256::new(kek.into());
	let mut wrapped = [0u8; WRAPPED_CEK_LEN];
	kw.wrap_key(cek, &mut wrapped)
		.map_err(|err| anyhow!("AES-KW wrap failed: {}", err))?;
	Ok(wrapped)
}

fn unwrap_cek(kek: &[u8; CEK_LEN], wrapped: &[u8]) -> anyhow::Result<[u8; CEK_LEN]> {
	let kw = KwAes256::new(kek.into());
	let mut cek = [0u8; CEK_LEN];
	kw.unwrap_key(wrapped, &mut cek)
		.map_err(|err| anyhow!("AES-KW unwrap failed: {}", err))?;
	Ok(cek)
}

fn party_v_info(kids: &[String]) -> [u8; 32] {
	let mut sorted = kids.to_vec();
	sorted.sort();
	let joined = sorted.join(".");
	Sha256::digest(joined.as_bytes()).into()
}

fn concat_kdf(z: &[u8], alg: &str, apu: Option<&[u8]>, apv: Option<&[u8]>) -> [u8; 32] {
	let mut other_info = Vec::new();
	other_info.extend(length_prefixed(alg.as_bytes()));
	other_info.extend(apu.map(length_prefixed).unwrap_or_else(|| vec![0, 0, 0, 0]));
	other_info.extend(apv.map(length_prefixed).unwrap_or_else(|| vec![0, 0, 0, 0]));
	other_info.extend([0, 0, 1, 0]);

	let mut input = vec![0, 0, 0, 1];
	input.extend(z);
	input.extend(other_info);
	Sha256::digest(&input).into()
}

fn length_prefixed(bytes: &[u8]) -> Vec<u8> {
	let mut result = (bytes.len() as u32).to_be_bytes().to_vec();
	result.extend(bytes);
	result
}

fn encode_json<T: Serialize>(value: &T) -> anyhow::Result<String> {
	Ok(encode_b64(serde_json::to_vec(value)?))
}

fn decode_json<T: DeserializeOwned>(value: &str) -> anyhow::Result<T> {
	Ok(serde_json::from_slice(&decode_b64(value)?)?)
}

fn encode_b64(bytes: impl AsRef<[u8]>) -> String {
	URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_b64(value: &str) -> anyhow::Result<Vec<u8>> {
	URL_SAFE_NO_PAD
		.decode(value)
		.map_err(|err| anyhow!("invalid base64url: {}", err))
}

#[cfg(test)]
mod tests {
	use super::{open, seal};
	use crate::{DidKeyIdentity, Identity};
	use base64::Engine as _;
	use serde_json::Value;

	#[test]
	fn envelope_round_trips_without_sender_metadata() {
		let to = DidKeyIdentity::generate_x25519(Some(&[31; 32]));
		let from = DidKeyIdentity::generate_x25519(Some(&[32; 32]));
		let plaintext = br#"{"id":"1","type":"test","body":"ok"}"#;

		let packed = seal(plaintext, to.identity(), &to.public_key_bytes()).unwrap();
		assert!(!packed.contains(from.identity()));
		let unpacked = open(&packed, to.private_key_bytes().divulge()).unwrap();
		assert_eq!(unpacked, plaintext);
	}

	#[test]
	fn envelope_has_didcomm_v2_anoncrypt_headers() {
		let to = DidKeyIdentity::generate_x25519(Some(&[33; 32]));
		let packed = seal(br#"{"id":"1","type":"test","body":{}}"#, to.identity(), &to.public_key_bytes()).unwrap();
		let envelope: Value = serde_json::from_str(&packed).unwrap();
		let protected = envelope.get("protected").and_then(Value::as_str).unwrap();
		let protected = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(protected).unwrap();
		let protected: Value = serde_json::from_slice(&protected).unwrap();

		assert_eq!(protected.get("typ").and_then(Value::as_str), Some("application/didcomm-encrypted+json"));
		assert_eq!(protected.get("alg").and_then(Value::as_str), Some("ECDH-ES+A256KW"));
		assert_eq!(protected.get("enc").and_then(Value::as_str), Some("XC20P"));
		assert!(protected.get("skid").is_none());
		assert!(protected.get("apu").is_none());
		assert!(protected.get("epk").is_some());
		assert!(protected.get("apv").is_some());
	}
}
