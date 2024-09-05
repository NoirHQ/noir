// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

use crate::{
	traits::{Checkable, VerifyMut},
	MultiSigner,
};
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sp_core::{ecdsa, ed25519, sr25519};
use sp_runtime::{
	traits::{IdentifyAccount, Lazy, Verify},
	RuntimeDebug,
};

/// Signature verify that can work with any known signature types.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub enum MultiSignature {
	/// An Ed25519 signature.
	Ed25519(ed25519::Signature),
	/// An Sr25519 signature.
	Sr25519(sr25519::Signature),
	/// An ECDSA/SECP256k1 signature.
	Ecdsa(ecdsa::Signature),
}

impl From<ed25519::Signature> for MultiSignature {
	fn from(x: ed25519::Signature) -> Self {
		Self::Ed25519(x)
	}
}

impl TryFrom<MultiSignature> for ed25519::Signature {
	type Error = ();
	fn try_from(m: MultiSignature) -> Result<Self, Self::Error> {
		if let MultiSignature::Ed25519(x) = m {
			Ok(x)
		} else {
			Err(())
		}
	}
}

impl From<sr25519::Signature> for MultiSignature {
	fn from(x: sr25519::Signature) -> Self {
		Self::Sr25519(x)
	}
}

impl TryFrom<MultiSignature> for sr25519::Signature {
	type Error = ();
	fn try_from(m: MultiSignature) -> Result<Self, Self::Error> {
		if let MultiSignature::Sr25519(x) = m {
			Ok(x)
		} else {
			Err(())
		}
	}
}

impl From<ecdsa::Signature> for MultiSignature {
	fn from(x: ecdsa::Signature) -> Self {
		Self::Ecdsa(x)
	}
}

impl TryFrom<MultiSignature> for ecdsa::Signature {
	type Error = ();
	fn try_from(m: MultiSignature) -> Result<Self, Self::Error> {
		if let MultiSignature::Ecdsa(x) = m {
			Ok(x)
		} else {
			Err(())
		}
	}
}

impl Verify for MultiSignature {
	type Signer = MultiSigner;
	fn verify<L: Lazy<[u8]>>(
		&self,
		mut msg: L,
		signer: &<Self::Signer as IdentifyAccount>::AccountId,
	) -> bool {
		let who: [u8; 32] = **signer;
		match self {
			Self::Ed25519(sig) => sig.verify(msg, &who.into()),
			Self::Sr25519(sig) => sig.verify(msg, &who.into()),
			Self::Ecdsa(sig) => {
				let m = sp_io::hashing::blake2_256(msg.get());
				sp_io::crypto::secp256k1_ecdsa_recover_compressed(sig.as_ref(), &m)
					.map_or(false, |pubkey| sp_io::hashing::blake2_256(&pubkey) == who)
			},
		}
	}
}

impl VerifyMut for MultiSignature {
	type Signer = MultiSigner;

	fn verify_mut<L: Lazy<[u8]>>(
		&self,
		mut msg: L,
		signer: &mut <Self::Signer as IdentifyAccount>::AccountId,
	) -> bool {
		let who: [u8; 32] = **signer;
		match self {
			Self::Ed25519(sig) => {
				let who = who.into();
				sig.verify(msg, &who).then(|| signer.check(who)).unwrap_or(false)
			},
			Self::Sr25519(sig) => {
				let who = who.into();
				sig.verify(msg, &who).then(|| signer.check(who)).unwrap_or(false)
			},
			Self::Ecdsa(sig) => {
				let m = sp_io::hashing::blake2_256(msg.get());
				sp_io::crypto::secp256k1_ecdsa_recover_compressed(sig.as_ref(), &m)
					.map_or(false, |pubkey| signer.check(ecdsa::Public::from(pubkey)))
			},
		}
	}
}
