// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "cosmos")]
pub mod cosmos;
#[cfg(feature = "ethereum")]
pub mod ethereum;
pub mod extensions;

pub use extensions::unify_account::UnifyAccount;

#[cfg(feature = "cosmos")]
pub use np_cosmos::Address as CosmosAddress;
#[cfg(feature = "ethereum")]
pub use np_ethereum::Address as EthereumAddress;
use pallet_multimap::traits::UniqueMultimap;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::ecdsa;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum Address {
	#[cfg(feature = "cosmos")]
	Cosmos(CosmosAddress),
	#[cfg(feature = "ethereum")]
	Ethereum(EthereumAddress),
}

impl Address {
	pub const fn variant_count() -> u32 {
		let mut n = 0;
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
