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
use sp_core::ecdsa;

#[cfg(feature = "cosmos")]
pub use np_cosmos as cosmos;
#[cfg(feature = "cosmos")]
pub use np_cosmos::Address as CosmosAddress;
#[cfg(feature = "ethereum")]
pub use np_ethereum as ethereum;
#[cfg(feature = "ethereum")]
pub use np_ethereum::Address as EthereumAddress;
pub use sp_core::crypto::AccountId32;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Address {
	Polkadot(AccountId32),
	#[cfg(feature = "cosmos")]
	Cosmos(CosmosAddress),
	#[cfg(feature = "ethereum")]
	Ethereum(EthereumAddress),
}

impl Address {
	pub const fn variant_count() -> u32 {
		let mut n = 1;
		if cfg!(feature = "cosmos") {
			n += 1;
		}
		if cfg!(feature = "ethereum") {
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
}
