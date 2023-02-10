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

//! Cryptography extensions to Substrate

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod bip32;
pub mod p256;
pub mod webauthn;

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
pub use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sp_core::RuntimeDebug;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_std::{hash::Hash, vec::Vec};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Hash))]
pub struct UniversalAccountId(Vec<u8>);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum UniversalSigner {
	WebAuthn(crate::p256::Public),
}

impl IdentifyAccount for UniversalSigner {
	type AccountId = UniversalAccountId;

	fn into_account(self) -> Self::AccountId {
		match self {
			Self::WebAuthn(who) => UniversalAccountId(who.0.to_vec()),
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct AuthenticatorAssertionResponse {
	pub client_data_json: Vec<u8>,
	pub authenticator_data: Vec<u8>,
	// after decoding or before decoding by ASN.1?
	pub signature: crate::p256::Signature,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum UniversalSignature {
	WebAuthn(AuthenticatorAssertionResponse),
}

impl Verify for UniversalSignature {
	type Signer = UniversalSigner;

	fn verify<L: sp_runtime::traits::Lazy<[u8]>>(
		&self,
		mut msg: L,
		signer: &<Self::Signer as sp_runtime::traits::IdentifyAccount>::AccountId,
	) -> bool {
		match self {
			UniversalSignature::WebAuthn(res) => crate::webauthn::crypto::webauthn_es256_verify(
				&res.signature,
				msg.get(),
				res.client_data_json.as_slice(),
				res.authenticator_data.as_slice(),
				signer.0.as_slice(),
			),
		}
	}
}
