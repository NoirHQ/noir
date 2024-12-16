// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "cosmos", feature = "ethereum", feature = "nostr"))]
use sp_core::ecdsa;
#[cfg(feature = "solana")]
use sp_core::ed25519;

pub use sp_core::crypto::AccountId32;
#[cfg(feature = "cosmos")]
pub use {np_cosmos as cosmos, np_cosmos::Address as CosmosAddress};
#[cfg(feature = "ethereum")]
pub use {np_ethereum as ethereum, np_ethereum::Address as EthereumAddress};
#[cfg(feature = "nostr")]
pub use {np_nostr as nostr, np_nostr::Address as NostrAddress};
#[cfg(feature = "solana")]
pub use {np_solana as solana, np_solana::Address as SolanaAddress};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum VarAddress {
	Polkadot(AccountId32),
	#[cfg(feature = "cosmos")]
	Cosmos(CosmosAddress),
	#[cfg(feature = "ethereum")]
	Ethereum(EthereumAddress),
	#[cfg(feature = "nostr")]
	Nostr(NostrAddress),
	#[cfg(feature = "solana")]
	Solana(SolanaAddress),
}

impl VarAddress {
	pub const fn variant_count() -> u32 {
		let mut n = 1;
		if cfg!(feature = "cosmos") {
			n += 1;
		}
		if cfg!(feature = "ethereum") {
			n += 1;
		}
		if cfg!(feature = "nostr") {
			n += 1;
		}
		if cfg!(feature = "solana") {
			n += 1;
		}
		n
	}

	#[cfg(feature = "cosmos")]
	pub fn cosmos(public: ecdsa::Public) -> Self {
		Self::Cosmos(CosmosAddress::from(public))
	}

	#[cfg(feature = "ethereum")]
	pub fn ethereum(public: ecdsa::Public) -> Self {
		Self::Ethereum(EthereumAddress::from(public))
	}

	#[cfg(feature = "nostr")]
	pub fn nostr(public: ecdsa::Public) -> Self {
		Self::Nostr(NostrAddress::from(public))
	}

	#[cfg(feature = "solana")]
	pub fn solana(public: ed25519::Public) -> Self {
		Self::Solana(SolanaAddress::from(public))
	}
}
