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

//! Universal account infrastructure.

use codec::{Decode, Encode, EncodeLike, Input, MaxEncodedLen};
use np_crypto::{ecdsa::EcdsaExt, p256};
use scale_info::TypeInfo;
use sp_core::{ecdsa, ed25519, sr25519, H160, H256};
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use base64ct::{Base64UrlUnpadded as Base64, Encoding};
#[cfg(feature = "std")]
use serde::{
	de::{Deserialize, Deserializer, Error as DeError, Visitor},
	ser::{Serialize, Serializer},
};

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

/// The type of public key that universal address contains.
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, PartialEq, Eq)]
pub enum UniversalAddressKind {
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

/// A universal representation of a public key encoded with multicodec.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, TypeInfo)]
#[cfg_attr(feature = "std", derive(Hash))]
pub struct UniversalAddress(pub Vec<u8>);

#[cfg(feature = "std")]
impl Serialize for UniversalAddress {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let encoded = String::from("u") + &Base64::encode_string(&self.0);
		serializer.serialize_str(&encoded)
	}
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for UniversalAddress {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct UniversalAddressVisitor;

		impl<'de> Visitor<'de> for UniversalAddressVisitor {
			type Value = UniversalAddress;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("a multibase (base64url) encoded string")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: DeError,
			{
				use sp_std::str::FromStr;

				UniversalAddress::from_str(value)
					.map_err(|_| E::custom("invalid universal address"))
			}
		}

		deserializer.deserialize_str(UniversalAddressVisitor)
	}
}

impl UniversalAddress {
	/// Get the type of public key that contains.
	pub fn kind(&self) -> UniversalAddressKind {
		match &self.0[0..4] {
			[0xe7, 0x01, ..] => UniversalAddressKind::Secp256k1,
			[0xed, 0x01, ..] => UniversalAddressKind::Ed25519,
			[0xef, 0x01, ..] => UniversalAddressKind::Sr25519,
			[0x80, 0x24, ..] => UniversalAddressKind::P256,
			[0xa0, 0xe4, 0x02, 0x20] => UniversalAddressKind::Blake2b256,
			_ => UniversalAddressKind::Unknown,
		}
	}
}

impl EcdsaExt for UniversalAddress {
	fn to_eth_address(&self) -> Option<H160> {
		match self.kind() {
			UniversalAddressKind::Secp256k1 => {
				let pubkey =
					np_io::crypto::secp256k1_pubkey_serialize(&self.0[2..].try_into().unwrap())?;
				Some(H160::from_slice(&sp_io::hashing::keccak_256(&pubkey)[12..]))
			},
			_ => None,
		}
	}
}

impl AsRef<[u8]> for UniversalAddress {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}

impl AsMut<[u8]> for UniversalAddress {
	fn as_mut(&mut self) -> &mut [u8] {
		self.0.as_mut()
	}
}

impl TryFrom<&[u8]> for UniversalAddress {
	type Error = ();

	fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self(Vec::try_from(data).map_err(|_| ())?))
	}
}

impl MaxEncodedLen for UniversalAddress {
	fn max_encoded_len() -> usize {
		36
	}
}

impl Encode for UniversalAddress {
	fn size_hint(&self) -> usize {
		match self.kind() {
			UniversalAddressKind::Ed25519 => 34,
			UniversalAddressKind::Sr25519 => 34,
			UniversalAddressKind::Secp256k1 => 35,
			UniversalAddressKind::P256 => 35,
			UniversalAddressKind::Blake2b256 => 36,
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

impl EncodeLike for UniversalAddress {}

impl Decode for UniversalAddress {
	fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
		let byte = input.read_byte()?;
		let expected_len = match byte {
			0xed | 0xef => 34,
			0xe7 | 0x80 => 35,
			0xa0 => 36,
			_ => return Err("unexpected first byte decoding UniversalAddress".into()),
		};
		let mut res = Vec::new();
		res.resize(expected_len, 0);
		res[0] = byte;
		input.read(&mut res[1..])?;

		let res = UniversalAddress(res);
		match res.kind() {
			UniversalAddressKind::Unknown => Err("Could not decode UniversalAddress".into()),
			_ => Ok(res),
		}
	}
}

impl From<ed25519::Public> for UniversalAddress {
	fn from(k: ed25519::Public) -> Self {
		let mut v: Vec<u8> = Vec::new();
		v.extend_from_slice(multicodec::ED25519_PUB);
		v.extend_from_slice(k.as_ref());
		Self(v)
	}
}

impl From<sr25519::Public> for UniversalAddress {
	fn from(k: sr25519::Public) -> Self {
		let mut v: Vec<u8> = Vec::new();
		v.extend_from_slice(multicodec::SR25519_PUB);
		v.extend_from_slice(k.as_ref());
		Self(v)
	}
}

impl From<ecdsa::Public> for UniversalAddress {
	fn from(k: ecdsa::Public) -> Self {
		let mut v: Vec<u8> = Vec::new();
		v.extend_from_slice(multicodec::SECP256K1_PUB);
		v.extend_from_slice(k.as_ref());
		Self(v)
	}
}

impl From<p256::Public> for UniversalAddress {
	fn from(k: p256::Public) -> Self {
		let mut v: Vec<u8> = Vec::new();
		v.extend_from_slice(multicodec::P256_PUB);
		v.extend_from_slice(k.as_ref());
		Self(v)
	}
}

impl From<H256> for UniversalAddress {
	fn from(hash: H256) -> Self {
		let mut v: Vec<u8> = Vec::new();
		v.extend_from_slice(multicodec::BLAKE2B_256);
		v.extend_from_slice(hash.as_ref());
		Self(v)
	}
}

impl TryInto<ed25519::Public> for UniversalAddress {
	type Error = ();

	fn try_into(self) -> Result<ed25519::Public, Self::Error> {
		match &self.0[0..2] {
			multicodec::ED25519_PUB => Ok(ed25519::Public::try_from(&self.0[2..]).map_err(|_| ())?),
			_ => Err(()),
		}
	}
}

impl TryInto<sr25519::Public> for UniversalAddress {
	type Error = ();

	fn try_into(self) -> Result<sr25519::Public, Self::Error> {
		match &self.0[0..2] {
			multicodec::SR25519_PUB => Ok(sr25519::Public::try_from(&self.0[2..]).map_err(|_| ())?),
			_ => Err(()),
		}
	}
}

impl TryInto<ecdsa::Public> for UniversalAddress {
	type Error = ();

	fn try_into(self) -> Result<ecdsa::Public, Self::Error> {
		match &self.0[0..2] {
			multicodec::SECP256K1_PUB => Ok(ecdsa::Public::try_from(&self.0[2..]).map_err(|_| ())?),
			_ => Err(()),
		}
	}
}

impl TryInto<p256::Public> for UniversalAddress {
	type Error = ();

	fn try_into(self) -> Result<p256::Public, Self::Error> {
		match &self.0[0..2] {
			multicodec::P256_PUB => Ok(p256::Public::try_from(&self.0[2..]).map_err(|_| ())?),
			_ => Err(()),
		}
	}
}

#[cfg(feature = "std")]
impl sp_std::str::FromStr for UniversalAddress {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let addr = if s.starts_with('u') {
			UniversalAddress(Base64::decode_vec(&s[1..]).map_err(|_| ())?)
		} else if s.starts_with("0x") {
			UniversalAddress(array_bytes::hex2bytes(&s[2..]).map_err(|_| ())?)
		} else {
			return Err(())
		};

		match addr.kind() {
			UniversalAddressKind::Unknown => Err(()),
			_ => Ok(addr),
		}
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for UniversalAddress {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "u{}", Base64::encode_string(self.as_ref()))
	}
}

impl sp_std::fmt::Debug for UniversalAddress {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "u{}", Base64::encode_string(self.as_ref()))
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn universal_address_from_or_to_string() {
		use sp_std::str::FromStr;

		let k = array_bytes::hex2bytes(
			"023af1e1efa4d1e1ad5cb9e3967e98e901dafcd37c44cf0bfb6c216997f5ee51df",
		)
		.unwrap();

		let ua = UniversalAddress::from_str("u5wECOvHh76TR4a1cueOWfpjpAdr803xEzwv7bCFpl_XuUd8");
		assert!(ua.is_ok());
		let ua = ua.unwrap();
		assert_eq!(&ua.0[2..], k);
		assert_eq!(ua.to_string(), "u5wECOvHh76TR4a1cueOWfpjpAdr803xEzwv7bCFpl_XuUd8");
	}

	#[test]
	fn universal_address_kind() {
		let k = array_bytes::hex2bytes(
			"023af1e1efa4d1e1ad5cb9e3967e98e901dafcd37c44cf0bfb6c216997f5ee51df",
		)
		.unwrap();
		let k = ecdsa::Public::try_from(&k[..]).unwrap();
		let ua = UniversalAddress::from(k);
		assert_eq!(ua.kind(), UniversalAddressKind::Secp256k1);
	}
}
