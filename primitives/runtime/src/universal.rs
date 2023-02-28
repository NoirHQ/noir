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

use codec::{Decode, Encode, MaxEncodedLen};
use np_crypto::{p256, webauthn};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{IdentifyAccount, Lazy, Verify},
	RuntimeDebug,
};
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Multicodec codes encoded with unsigned varint.
#[allow(dead_code)]
pub mod multicodec {
	/// Multicodec code for Secp256k1 public key. (0xe7)
	pub const SECP256K1_PUB: [u8; 2] = [0xe7, 0x01];
	/// Multicodec code for Ed25519 public key. (0xed)
	pub const ED25519_PUB: [u8; 2] = [0xed, 0x01];
	/// Multicodec code for Sr25519 public key. (0xef)
	pub const SR25519_PUB: [u8; 2] = [0xef, 0x01];
	/// Multicodec code for P-256 public key. (0x1200)
	pub const P256_PUB: [u8; 2] = [0x80, 0x24];
}

/// A universal representation of a public key encoded with multicodec.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Hash, Serialize, Deserialize))]
pub struct UniversalAddress(pub Vec<u8>);

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
		35 // multicodec code (2) + compressed public key (33)
	}
}

impl From<p256::Public> for UniversalAddress {
	fn from(k: p256::Public) -> Self {
		let mut v: Vec<u8> = Vec::new();
		v.extend_from_slice(&multicodec::P256_PUB[..]);
		v.extend_from_slice(k.as_ref());
		Self(v)
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for UniversalAddress {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", array_bytes::bytes2hex("", self.as_ref()))
	}
}

impl sp_std::fmt::Debug for UniversalAddress {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "{}", array_bytes::bytes2hex("", self.as_ref()))
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

/// Signature verify that can work with any known signature types.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum UniversalSignature {
	/// A P-256 signature.
	P256(p256::Signature),
	/// A WebAuthn ES256 signature.
	WebAuthn(webauthn::Signature),
}

impl Verify for UniversalSignature {
	type Signer = UniversalSigner;

	fn verify<L: Lazy<[u8]>>(&self, mut msg: L, signer: &UniversalAddress) -> bool {
		match (self, signer) {
			(Self::P256(ref sig), who) => match p256::Public::try_from(who.as_ref()) {
				Ok(signer) => np_io::crypto::p256_verify(sig, msg.get(), &signer),
				Err(_) => false,
			},
			(Self::WebAuthn(ref sig), who) => match webauthn::Public::try_from(who.as_ref()) {
				Ok(signer) => np_io::crypto::webauthn_verify(sig, msg.get(), &signer),
				Err(_) => false,
			},
		}
	}
}

/// Public key for any known crypto algorithm.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum UniversalSigner {
	/// A P-256 identity.
	P256(p256::Public),
	/// A WebAuthn ES256 identity.
	WebAuthn(webauthn::Public),
}

impl IdentifyAccount for UniversalSigner {
	type AccountId = UniversalAddress;

	fn into_account(self) -> Self::AccountId {
		match self {
			Self::P256(k) => k.into(),
			Self::WebAuthn(k) => k.into(),
		}
	}
}

impl From<p256::Public> for UniversalSigner {
	fn from(k: p256::Public) -> Self {
		Self::P256(k)
	}
}
