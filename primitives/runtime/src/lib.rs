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

//! Noir Runtime Shareable types.

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

mod accountname;
mod universaladdress;

pub use accountname::AccountName;
pub use universaladdress::{UniversalAddress, UniversalAddressKind};

use np_crypto::{p256, webauthn};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{ecdsa, ed25519, sr25519};
use sp_runtime::{
	traits::{IdentifyAccount, Lazy, Verify},
	RuntimeDebug,
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Signature verify that can work with any known signature types.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum UniversalSignature {
	/// A Ed25519 signature.
	Ed25519(ed25519::Signature),
	/// A Sr25519 signature.
	Sr25519(sr25519::Signature),
	/// A Secp256k1 signature.
	Secp256k1(ecdsa::Signature),
	/// A P-256 signature.
	P256(p256::Signature),
	/// A WebAuthn ES256 signature.
	WebAuthn(webauthn::Signature),
}

impl Verify for UniversalSignature {
	type Signer = UniversalSigner;

	fn verify<L: Lazy<[u8]>>(&self, mut msg: L, signer: &UniversalAddress) -> bool {
		match (self, signer) {
			(Self::Ed25519(ref sig), who) => match ed25519::Public::try_from(&who.0[2..]) {
				Ok(signer) => sig.verify(msg, &signer),
				Err(()) => false,
			},
			(Self::Sr25519(ref sig), who) => match sr25519::Public::try_from(&who.0[2..]) {
				Ok(signer) => sig.verify(msg, &signer),
				Err(()) => false,
			},
			(Self::Secp256k1(ref sig), who) => match ecdsa::Public::try_from(&who.0[2..]) {
				Ok(signer) => {
					let m = sp_io::hashing::blake2_256(msg.get());
					match sp_io::crypto::secp256k1_ecdsa_recover_compressed(sig.as_ref(), &m) {
						Ok(pubkey) => pubkey == signer.0,
						_ => false,
					}
				},
				Err(_) => false,
			},
			(Self::P256(ref sig), who) => match p256::Public::try_from(&who.0[2..]) {
				Ok(signer) => np_io::crypto::p256_verify(sig, msg.get(), &signer),
				Err(_) => false,
			},
			(Self::WebAuthn(ref sig), who) => match p256::Public::try_from(&who.0[2..]) {
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
	/// A Ed25519 identity.
	Ed25519(ed25519::Public),
	/// A Sr25519 identity.
	Sr25519(sr25519::Public),
	/// A Secp256k1 identity.
	Secp256k1(ecdsa::Public),
	/// A P-256 identity.
	P256(p256::Public),
}

impl IdentifyAccount for UniversalSigner {
	type AccountId = UniversalAddress;

	fn into_account(self) -> Self::AccountId {
		match self {
			Self::Ed25519(k) => k.into(),
			Self::Sr25519(k) => k.into(),
			Self::Secp256k1(k) => k.into(),
			Self::P256(k) => k.into(),
		}
	}
}

impl From<ed25519::Public> for UniversalSigner {
	fn from(k: ed25519::Public) -> Self {
		Self::Ed25519(k)
	}
}

impl From<sr25519::Public> for UniversalSigner {
	fn from(k: sr25519::Public) -> Self {
		Self::Sr25519(k)
	}
}

impl From<ecdsa::Public> for UniversalSigner {
	fn from(k: ecdsa::Public) -> Self {
		Self::Secp256k1(k)
	}
}

impl From<p256::Public> for UniversalSigner {
	fn from(k: p256::Public) -> Self {
		Self::P256(k)
	}
}
