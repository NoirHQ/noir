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

//! Simple WebAuthn API.

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::crypto::ByteArray;
use sp_runtime_interface::pass_by::{PassByCodec, PassByInner};
use sp_std::vec::Vec;

use crate::p256;

#[cfg(feature = "full_crypto")]
use base64ct::{Base64UrlUnpadded as Base64, Encoding};
#[cfg(feature = "full_crypto")]
use sp_core::hashing::sha2_256;

#[cfg(feature = "std")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "std")]
use sp_core::crypto::Ss58Codec;

/// Webauthn es256 public key.
#[cfg_attr(feature = "full_crypto", derive(Hash))]
#[derive(
	Clone,
	Copy,
	Encode,
	Decode,
	PassByInner,
	MaxEncodedLen,
	TypeInfo,
	Eq,
	PartialEq,
	PartialOrd,
	Ord,
)]
pub struct Public(pub p256::Public);

impl ByteArray for Public {
	const LEN: usize = p256::Public::LEN;
}

impl AsRef<[u8]> for Public {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}

impl AsMut<[u8]> for Public {
	fn as_mut(&mut self) -> &mut [u8] {
		self.0.as_mut()
	}
}

impl TryFrom<&[u8]> for Public {
	type Error = ();

	fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self(p256::Public::try_from(data)?))
	}
}

#[cfg(feature = "std")]
impl Ss58Codec for Public {}

#[cfg(feature = "std")]
impl std::fmt::Display for Public {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.to_ss58check())
	}
}

impl sp_std::fmt::Debug for Public {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let s = self.to_ss58check();
		write!(f, "{} ({}...)", sp_core::hexdisplay::HexDisplay::from(&self.as_ref()), &s[0..8])
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

#[cfg(feature = "std")]
impl Serialize for Public {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.to_ss58check())
	}
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for Public {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Public::from_ss58check(&String::deserialize(deserializer)?)
			.map_err(|e| de::Error::custom(format!("{:?}", e)))
	}
}

#[cfg(feature = "full_crypto")]
#[derive(Serialize, Deserialize, Debug)]
struct ClientDataJson {
	#[serde(alias = "type")]
	pub type_: String,
	pub challenge: String,
	pub origin: String,
}

#[cfg(feature = "full_crypto")]
impl ClientDataJson {
	// challenge should be same to the hash value of the message.
	fn check_message(&self, message_hash: &[u8]) -> bool {
		match Base64::decode_vec(&self.challenge) {
			Ok(c) => c == message_hash,
			Err(_) => false,
		}
	}

	// origin should be same to the rpId or its subdomain.
	fn check_rpid(&self, rpid_hash: &[u8]) -> bool {
		let mut rpid = &self.origin["https://".len()..];
		rpid = match rpid.rfind(':') {
			Some(pos) => &rpid[..pos],
			None => rpid,
		};
		while sha2_256(rpid.as_ref()) != rpid_hash {
			// registrable domain suffix
			match rpid.split_once('.') {
				Some((_, rds)) => rpid = rds,
				None => return false,
			};
		}
		true
	}
}

#[cfg(feature = "full_crypto")]
impl TryFrom<&str> for ClientDataJson {
	type Error = ();

	fn try_from(s: &str) -> Result<Self, Self::Error> {
		Self::try_from(Base64::decode_vec(s).map_err(|_| ())?.as_slice())
	}
}

#[cfg(feature = "full_crypto")]
impl TryFrom<&[u8]> for ClientDataJson {
	type Error = ();

	fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
		let client_data: ClientDataJson = match serde_json::from_slice(data) {
			Ok(c) => c,
			Err(_) => return Err(()),
		};
		if client_data.type_ != "webauthn.get" {
			return Err(())
		}
		if !client_data.origin.starts_with("https://") {
			return Err(())
		}
		Ok(client_data)
	}
}

/// Webauthn es256 signature. This type corresponds to AuthenticatorAssertionResponse.
#[cfg_attr(feature = "full_crypto", derive(Hash))]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Encode, Decode, TypeInfo, PassByCodec, PartialEq, Eq)]
pub struct Signature {
	/// Client data passed to the authenticator to generate a signature.
	pub client_data_json: Vec<u8>,
	/// Contextual bindings made by the authenticator.
	pub authenticator_data: Vec<u8>,
	/// Signature generated by the authenticator. (ASN.1 DER encoded)
	pub signature: Vec<u8>,
}

#[cfg(feature = "full_crypto")]
impl Signature {
	/// Verify a signature on a message. Returns true if the signature is good.
	pub fn verify<M: AsRef<[u8]>>(&self, message: M, pubkey: &Public) -> bool {
		self.verify_prehashed(&sha2_256(message.as_ref()), pubkey)
	}

	/// Verify a signature on a pre-hashed message. Return `true` if the signature is valid
	/// and thus matches the given `public` key.
	pub fn verify_prehashed(&self, message_hash: &[u8; 32], pubkey: &Public) -> bool {
		let client_data = match ClientDataJson::try_from(&self.client_data_json[..]) {
			Ok(c) => c,
			Err(_) => return false,
		};
		if !client_data.check_message(message_hash) {
			return false
		}
		if self.authenticator_data.len() < 37 {
			return false
		}
		if !client_data.check_rpid(self.rpid_hash()) {
			return false
		}
		let mut signed_message: Vec<u8> = Vec::new();
		signed_message.extend(&self.authenticator_data);
		signed_message.extend(sha2_256(&self.client_data_json));
		match p256::Signature::from_der(&self.signature[..]) {
			Some(sig) =>
				p256::Pair::verify_prehashed(&sig, &sha2_256(&signed_message[..]), &pubkey.0),
			None => false,
		}
	}

	// WARNING: This function doesn't check the size of authenticator data.
	fn rpid_hash(&self) -> &[u8] {
		&self.authenticator_data[0..32]
	}
}

impl sp_std::fmt::Debug for Signature {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.debug_struct("WebAuthnSignature")
			.field("clientDataJson", &self.client_data_json)
			.field("authenticatorData", &self.authenticator_data)
			.field("signature", &self.signature)
			.finish()
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
	fn parse_client_data_json() {
		let client_data_json = "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiT212TVJpd0dPMW5oZy1WdXBrUUZ0bVJybUp6MEUwdTlnNXc1RWtpYnctVEhmYkJ4dUx4ek5PMVgtVmd6cnVWNDloQ3ZyMGIxT2NuQnJBOUVfaWFhREEiLCJvcmlnaW4iOiJodHRwczovL3dlYmF1dGhuLmlvIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ";
		let client_data = ClientDataJson::try_from(client_data_json);
		assert!(client_data.is_ok());

		let client_data_json = Base64::decode_vec(client_data_json).unwrap();
		let client_data = ClientDataJson::try_from(client_data_json.as_slice());
		assert!(client_data.is_ok());

		let client_data = client_data.unwrap();
		assert_eq!(client_data.type_, "webauthn.get".to_string());
		assert_eq!(client_data.challenge, "OmvMRiwGO1nhg-VupkQFtmRrmJz0E0u9g5w5Ekibw-THfbBxuLxzNO1X-VgzruV49hCvr0b1OcnBrA9E_iaaDA".to_string());
		assert_eq!(client_data.origin, "https://webauthn.io".to_string());
	}

	#[test]
	fn check_rpid() {
		let client_data_json = "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiT212TVJpd0dPMW5oZy1WdXBrUUZ0bVJybUp6MEUwdTlnNXc1RWtpYnctVEhmYkJ4dUx4ek5PMVgtVmd6cnVWNDloQ3ZyMGIxT2NuQnJBOUVfaWFhREEiLCJvcmlnaW4iOiJodHRwczovL3dlYmF1dGhuLmlvIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ";
		let client_data = ClientDataJson::try_from(client_data_json).unwrap();

		let authenticator_data = "dKbqkhPJnC90siSSsyDPQCYqlMGpUKA5fyklC2CEHvABAAAAAg";
		let authenticator_data = Base64::decode_vec(authenticator_data).unwrap();
		let rpid_hash = &authenticator_data[0..32];
		assert!(client_data.check_rpid(rpid_hash));
	}

	#[test]
	fn verify_signature() {
		let public = array_bytes::hex2bytes_unchecked(
			"03a6df82984f9a6f67b5c46424fac587ba40dcf58dcf279aa635266e9382e31db3",
		);
		let public = Public::try_from(public.as_ref()).unwrap();
		let signature = Signature {
			signature: Base64::decode_vec("MEQCIHjqOGStreQAKH44uq5lQL5afSdZAhaOKwvnPdpPPfZiAiB-piO5KWYcYDXbvHWIXQirbN1Ww5sLfIvCGGyE1qOdtg").unwrap(),
			authenticator_data: Base64::decode_vec("dKbqkhPJnC90siSSsyDPQCYqlMGpUKA5fyklC2CEHvAFAAAABA").unwrap(),
			client_data_json: Base64::decode_vec("eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQSIsIm9yaWdpbiI6Imh0dHBzOi8vd2ViYXV0aG4uaW8iLCJjcm9zc09yaWdpbiI6ZmFsc2V9").unwrap(),
		};
		assert!(signature.verify_prehashed(&[0u8; 32], &public));
	}
}
