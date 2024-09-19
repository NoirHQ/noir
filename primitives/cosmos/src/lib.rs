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

//! Noir primitive types for Cosmos compatibility.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod traits;

#[cfg(feature = "std")]
use crate::traits::ChainInfo;
use crate::traits::CosmosHub;
#[cfg(feature = "std")]
use bech32::{Bech32, Hrp};
use buidl::FixedBytes;
use core::marker::PhantomData;
use parity_scale_codec::{Decode, Encode};
use ripemd::{Digest, Ripemd160};
use sp_core::{ecdsa, H160, H256};
use sp_io::hashing::{blake2_256, sha2_256};
use sp_runtime::traits::AccountIdConversion;

/// Cosmos address.
#[derive(FixedBytes)]
#[buidl(substrate(Core, Codec, TypeInfo))]
pub struct Address<T = CosmosHub>([u8; 20], PhantomData<fn() -> T>);

impl<T> From<H160> for Address<T> {
	fn from(h: H160) -> Self {
		Self(h.0, PhantomData)
	}
}

impl<T> From<Address<T>> for H160 {
	fn from(v: Address<T>) -> Self {
		H160(v.0)
	}
}

impl<T> From<ecdsa::Public> for Address<T> {
	fn from(key: ecdsa::Public) -> Self {
		let hash = sha2_256(&key.0);
		let hash = Ripemd160::digest(hash);
		Self(hash.into(), PhantomData)
	}
}

#[cfg(feature = "std")]
impl<T: ChainInfo> std::fmt::Display for Address<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let hrp = Hrp::parse_unchecked(T::bech32_prefix());
		write!(f, "{}", bech32::encode::<Bech32>(hrp, &self.0).expect("bech32 encode"))
	}
}

impl<T> core::fmt::Debug for Address<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "{}", sp_core::hexdisplay::HexDisplay::from(&self.0))
	}
}

impl<AccountId: From<H256>> AccountIdConversion<AccountId> for Address {
	fn into_account_truncating(&self) -> AccountId {
		let mut data = [0u8; 25];
		data[0..5].copy_from_slice(b"cosm:");
		data[5..25].copy_from_slice(&self.0);
		H256(blake2_256(&data)).into()
	}

	fn into_sub_account_truncating<S: Encode>(&self, _: S) -> AccountId {
		unimplemented!()
	}

	fn try_into_sub_account<S: Encode>(&self, _: S) -> Option<AccountId> {
		unimplemented!()
	}

	fn try_from_sub_account<S: Decode>(_: &AccountId) -> Option<(Self, S)> {
		unimplemented!()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn dev_public() -> ecdsa::Public {
		const_hex::decode_to_array(
			b"02509540919faacf9ab52146c9aa40db68172d83777250b28e4679176e49ccdd9f",
		)
		.unwrap()
		.into()
	}

	#[test]
	fn display_cosmos_address() {
		let address: Address = dev_public().into();
		assert_eq!(address.to_string(), "cosmos13essdahf3eajr07lhlpaawswmmfg5pr6t459pg");
	}
}
