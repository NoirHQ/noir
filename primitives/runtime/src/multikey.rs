// This file is part of Noir.

// Copyright (C) 2023 Haderech Pte. Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Interoperable public key representation.

use crate::AccountId32;
use np_crypto::{ecdsa::EcdsaExt, p256};
use parity_scale_codec::{Decode, Encode, EncodeLike, Error as CodecError, Input, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{
	crypto::{PublicError, UncheckedFrom},
	ecdsa, ed25519, sr25519, ByteArray, H160, H256,
};
use sp_runtime::traits::IdentifyAccount;
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

#[cfg(feature = "serde")]
use base64ct::{Base64UrlUnpadded, Encoding};
#[cfg(feature = "serde")]
use serde::{
	de::{Deserializer, Error as DeError, Visitor},
	ser::Serializer,
	Deserialize, Serialize,
};
#[cfg(all(not(feature = "std"), feature = "serde"))]
use sp_std::alloc::string::String;

/// Multicodec codes encoded with unsigned varint.
#[allow(dead_code)]
pub mod multicodec {
	/// Multicodec code for Secp256k1 public key. (0xe7)
	pub const SECP256K1_PUB: &[u8] = &[0xe7, 0x01];
	/// Multicodec code for Ed25519 public key. (0xed)
	pub const ED25519_PUB: &[u8] = &[0xed, 0x01];
	/// Multicodec code for Sr25519 public key. (0xef)
	pub const SR25519_PUB: &[u8] = &[0xef, 0x01];
	/// Multicodec code for P-256 public key. (0x1200)
	pub const P256_PUB: &[u8] = &[0x80, 0x24];
	/// Multicodec code for Blake2b-256 hash. (0xb220 + length(32))
	pub const BLAKE2B_256: &[u8] = &[0xa0, 0xe4, 0x02, 0x20];
}

/// The type of public key that multikey contains.
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, PartialEq, Eq)]
pub enum MultikeyKind {
	/// Unknown public key type. (Invalid)
	Unknown,
	/// Ed25519 public key type.
	Ed25519,
	/// Sr25519 public key type.
	Sr25519,
	/// Secp256k1 public key type.
	Secp256k1,
	/// P256 public key type.
	P256,
	/// BLAKE2b-256 hash value address for non-verifiable address.
	Blake2b256,
}

#[cfg_attr(feature = "std", derive(thiserror::Error))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
	#[cfg_attr(feature = "std", error("invalid length"))]
	BadLength,
	#[cfg_attr(feature = "std", error("invalid multicodec prefix"))]
	InvalidPrefix,
}

/// A universal representation of a public key encoded with multicodec.
///
/// NOTE: https://www.w3.org/TR/vc-data-integrity/#multikey
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, TypeInfo)]
#[cfg_attr(feature = "std", derive(Hash))]
pub struct Multikey(Vec<u8>);

impl IdentifyAccount for Multikey {
	type AccountId = AccountId32;

	fn into_account(self) -> Self::AccountId {
		match self.kind() {
			MultikeyKind::Ed25519 | MultikeyKind::Sr25519 =>
				<[u8; 32]>::try_from(&self.0[2..]).unwrap().into(),
			MultikeyKind::Secp256k1 | MultikeyKind::P256 =>
				sp_io::hashing::blake2_256(&self.0[2..]).into(),
			MultikeyKind::Blake2b256 => <[u8; 32]>::try_from(&self.0[4..]).unwrap().into(),
			_ => panic!("invalid multikey"),
		}
	}
}

#[cfg(feature = "serde")]
impl Serialize for Multikey {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let encoded = String::from("u") + &Base64UrlUnpadded::encode_string(&self.0);
		serializer.serialize_str(&encoded)
	}
}

// TODO: Support other multibase formats other than base64url.
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Multikey {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct MultikeyVisitor;

		impl<'de> Visitor<'de> for MultikeyVisitor {
			type Value = Multikey;

			fn expecting(&self, formatter: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
				formatter.write_str("a multibase (base64url) encoded string")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: DeError,
			{
				use sp_std::str::FromStr;

				Multikey::from_str(value).map_err(|_| E::custom("invalid multikey"))
			}
		}

		deserializer.deserialize_str(MultikeyVisitor)
	}
}

impl Multikey {
	/// Get the type of public key that contains.
	pub fn kind(&self) -> MultikeyKind {
		match &self.0[0..4] {
			[0xe7, 0x01, ..] => MultikeyKind::Secp256k1,
			[0xed, 0x01, ..] => MultikeyKind::Ed25519,
			[0xef, 0x01, ..] => MultikeyKind::Sr25519,
			[0x80, 0x24, ..] => MultikeyKind::P256,
			[0xa0, 0xe4, 0x02, 0x20] => MultikeyKind::Blake2b256,
			_ => MultikeyKind::Unknown,
		}
	}
}

impl EcdsaExt for Multikey {
	fn to_eth_address(&self) -> Option<H160> {
		match self.kind() {
			MultikeyKind::Secp256k1 => {
				let pubkey =
					np_io::crypto::secp256k1_pubkey_serialize(&self.0[2..].try_into().unwrap())?;
				Some(H160::from_slice(&sp_io::hashing::keccak_256(&pubkey)[12..]))
			},
			_ => None,
		}
	}

	fn to_cosm_address(&self) -> Option<H160> {
		match self.kind() {
			MultikeyKind::Secp256k1 => {
				let hashed = sp_io::hashing::sha2_256(&self.0[2..]);
				Some(np_io::crypto::ripemd160(&hashed).into())
			},
			_ => None,
		}
	}
}

impl AsRef<[u8]> for Multikey {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}

impl AsMut<[u8]> for Multikey {
	fn as_mut(&mut self) -> &mut [u8] {
		self.0.as_mut()
	}
}

impl TryFrom<&[u8]> for Multikey {
	type Error = Error;

	fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self::try_from(Vec::from(data))?)
	}
}

impl TryFrom<Vec<u8>> for Multikey {
	type Error = Error;

	fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
		if v.len() > 0 && v.len() < 34 {
			return Err(Error::BadLength);
		}
		match &v[0..4] {
			[0xe7, 0x01, ..] =>
				ecdsa::Public::try_from(&v[2..]).map_err(|_| Error::BadLength).map(Into::into),
			[0xed, 0x01, ..] =>
				ed25519::Public::try_from(&v[2..]).map_err(|_| Error::BadLength).map(Into::into),
			[0xef, 0x01, ..] =>
				sr25519::Public::try_from(&v[2..]).map_err(|_| Error::BadLength).map(Into::into),
			[0x80, 0x24, ..] =>
				p256::Public::try_from(&v[2..]).map_err(|_| Error::BadLength).map(Into::into),
			[0xa0, 0xe4, 0x02, 0x20] =>
				(v.len() == 36).then(|| Self(Vec::from(&v[4..]))).ok_or(Error::BadLength),
			_ => Err(Error::InvalidPrefix),
		}
	}
}

impl UncheckedFrom<Vec<u8>> for Multikey {
	fn unchecked_from(v: Vec<u8>) -> Self {
		Self(v)
	}
}

impl MaxEncodedLen for Multikey {
	fn max_encoded_len() -> usize {
		36
	}
}

impl Encode for Multikey {
	fn size_hint(&self) -> usize {
		match self.kind() {
			MultikeyKind::Ed25519 => 34,
			MultikeyKind::Sr25519 => 34,
			MultikeyKind::Secp256k1 => 35,
			MultikeyKind::P256 => 35,
			MultikeyKind::Blake2b256 => 36,
			_ => 0,
		}
	}

	fn encode(&self) -> Vec<u8> {
		self.0.clone()
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		f(&self.0)
	}
}

impl EncodeLike for Multikey {}

impl Decode for Multikey {
	fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
		let byte = input.read_byte()?;
		let expected_len = match byte {
			0xed | 0xef => 34,
			0xe7 | 0x80 => 35,
			0xa0 => 36,
			_ => return Err("unexpected first byte decoding Multikey".into()),
		};
		let mut res = Vec::new();
		res.resize(expected_len, 0);
		res[0] = byte;
		input.read(&mut res[1..])?;

		let res = Multikey(res);
		match res.kind() {
			MultikeyKind::Unknown => Err("Could not decode Multikey".into()),
			_ => Ok(res),
		}
	}
}

impl From<ed25519::Public> for Multikey {
	fn from(k: ed25519::Public) -> Self {
		let mut v: Vec<u8> =
			Vec::with_capacity(multicodec::ED25519_PUB.len() + ed25519::Public::LEN);
		v.extend_from_slice(multicodec::ED25519_PUB);
		v.extend_from_slice(k.as_ref());
		Self(v)
	}
}

impl From<sr25519::Public> for Multikey {
	fn from(k: sr25519::Public) -> Self {
		let mut v: Vec<u8> =
			Vec::with_capacity(multicodec::SR25519_PUB.len() + sr25519::Public::LEN);
		v.extend_from_slice(multicodec::SR25519_PUB);
		v.extend_from_slice(k.as_ref());
		Self(v)
	}
}

impl From<ecdsa::Public> for Multikey {
	fn from(k: ecdsa::Public) -> Self {
		let mut v: Vec<u8> =
			Vec::with_capacity(multicodec::SECP256K1_PUB.len() + ecdsa::Public::LEN);
		v.extend_from_slice(multicodec::SECP256K1_PUB);
		v.extend_from_slice(k.as_ref());
		Self(v)
	}
}

impl From<p256::Public> for Multikey {
	fn from(k: p256::Public) -> Self {
		let mut v: Vec<u8> = Vec::with_capacity(multicodec::P256_PUB.len() + p256::Public::LEN);
		v.extend_from_slice(multicodec::P256_PUB);
		v.extend_from_slice(k.as_ref());
		Self(v)
	}
}

impl From<H256> for Multikey {
	fn from(hash: H256) -> Self {
		let mut v: Vec<u8> = Vec::with_capacity(multicodec::BLAKE2B_256.len() + H256::len_bytes());
		v.extend_from_slice(multicodec::BLAKE2B_256);
		v.extend_from_slice(hash.as_ref());
		Self(v)
	}
}

impl TryFrom<AccountId32> for Multikey {
	type Error = ();

	fn try_from(v: AccountId32) -> Result<Self, Self::Error> {
		v.source().ok_or(()).cloned()
	}
}

impl TryFrom<Multikey> for ed25519::Public {
	type Error = PublicError;

	fn try_from(v: Multikey) -> Result<Self, Self::Error> {
		if v.kind() == MultikeyKind::Ed25519 {
			ed25519::Public::try_from(&v.0[2..]).map_err(|_| PublicError::BadLength)
		} else {
			Err(PublicError::InvalidPrefix)
		}
	}
}

impl TryFrom<Multikey> for sr25519::Public {
	type Error = PublicError;

	fn try_from(v: Multikey) -> Result<Self, Self::Error> {
		if v.kind() == MultikeyKind::Sr25519 {
			sr25519::Public::try_from(&v.0[2..]).map_err(|_| PublicError::BadLength)
		} else {
			Err(PublicError::InvalidPrefix)
		}
	}
}

impl TryFrom<Multikey> for ecdsa::Public {
	type Error = PublicError;

	fn try_from(v: Multikey) -> Result<Self, Self::Error> {
		if v.kind() == MultikeyKind::Secp256k1 {
			ecdsa::Public::try_from(&v.0[2..]).map_err(|_| PublicError::BadLength)
		} else {
			Err(PublicError::InvalidPrefix)
		}
	}
}

impl TryFrom<Multikey> for p256::Public {
	type Error = PublicError;

	fn try_from(v: Multikey) -> Result<Self, Self::Error> {
		if v.kind() == MultikeyKind::P256 {
			p256::Public::try_from(&v.0[2..]).map_err(|_| PublicError::BadLength)
		} else {
			Err(PublicError::InvalidPrefix)
		}
	}
}

#[cfg(feature = "serde")]
impl sp_std::str::FromStr for Multikey {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let addr = if s.starts_with('u') {
			Multikey(Base64UrlUnpadded::decode_vec(&s[1..]).map_err(|_| ())?)
		} else if s.starts_with("0x") {
			Multikey(array_bytes::hex2bytes(&s[2..]).map_err(|_| ())?)
		} else {
			return Err(())
		};

		match addr.kind() {
			MultikeyKind::Unknown => Err(()),
			_ => Ok(addr),
		}
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for Multikey {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "u{}", Base64UrlUnpadded::encode_string(self.as_ref()))
	}
}

impl sp_std::fmt::Debug for Multikey {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "u{}", Base64UrlUnpadded::encode_string(self.as_ref()))
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use parity_scale_codec::{Decode, Encode, IoReader};
	use sp_std::str::FromStr;

	#[test]
	fn string_serialization_works() {
		let raw = array_bytes::hex2bytes(
			"e701023af1e1efa4d1e1ad5cb9e3967e98e901dafcd37c44cf0bfb6c216997f5ee51df",
		)
		.unwrap();

		let addr = Multikey::from_str("u5wECOvHh76TR4a1cueOWfpjpAdr803xEzwv7bCFpl_XuUd8");
		assert!(addr.is_ok());

		let addr = addr.unwrap();
		assert_eq!(&addr.0[..], raw);
		assert_eq!(addr.to_string(), "u5wECOvHh76TR4a1cueOWfpjpAdr803xEzwv7bCFpl_XuUd8");
	}

	#[test]
	fn kind_corresponds_to_contained_public_key() {
		let pubkey = array_bytes::hex2bytes(
			"023af1e1efa4d1e1ad5cb9e3967e98e901dafcd37c44cf0bfb6c216997f5ee51df",
		)
		.unwrap();
		let pubkey = ecdsa::Public::try_from(&pubkey[..]).unwrap();
		let addr = Multikey::from(pubkey);
		assert_eq!(addr.kind(), MultikeyKind::Secp256k1);
	}

	#[test]
	fn scale_serialization_works() {
		let raw = array_bytes::hex2bytes(
			"e701023af1e1efa4d1e1ad5cb9e3967e98e901dafcd37c44cf0bfb6c216997f5ee51df",
		)
		.unwrap();
		let addr = Multikey(raw.clone());
		assert_eq!(addr.kind(), MultikeyKind::Secp256k1);

		let encoded = addr.encode();
		assert_eq!(encoded, raw);

		let mut io = IoReader(&encoded[..]);
		let decoded = Multikey::decode(&mut io);
		assert!(decoded.is_ok());
		assert_eq!(decoded.unwrap(), addr);
	}
}
