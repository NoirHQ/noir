// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
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

//! Noir primitive types for Ethereum compatibility.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "serde")]
use alloc::string::String;
use buidl::FixedBytes;
use k256::{elliptic_curve::sec1::ToEncodedPoint, PublicKey};
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sp_core::{ecdsa, H160, H256};
use sp_io::hashing::{blake2_256, keccak_256};
use sp_runtime::traits::AccountIdConversion;

/// Ethereum address.
#[derive(FixedBytes)]
#[buidl(derive(Substrate), skip_derive(PassBy))]
pub struct Address([u8; 20]);

impl From<H160> for Address {
	fn from(h: H160) -> Self {
		Self(h.0)
	}
}

impl From<Address> for H160 {
	fn from(v: Address) -> Self {
		Self(v.0)
	}
}

impl From<ecdsa::Public> for Address {
	fn from(key: ecdsa::Public) -> Self {
		PublicKey::from_sec1_bytes(&key.0)
			.map(|key| {
				let hash = keccak_256(&key.to_encoded_point(false).as_bytes()[1..]);
				let mut result = [0u8; 20];
				result.copy_from_slice(&hash[12..]);
				Address(result)
			})
			.expect("invalid ecdsa public key.")
	}
}

#[cfg(feature = "serde")]
impl core::fmt::Display for Address {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		use alloc::string::ToString;
		let address = const_hex::encode(self.0);
		let address_hash = const_hex::encode(keccak_256(address.as_bytes()));

		let checksum: String =
			address
				.char_indices()
				.fold(String::from("0x"), |mut acc, (index, address_char)| {
					let n = u16::from_str_radix(&address_hash[index..index + 1], 16)
						.expect("Keccak256 hashed; qed");

					if n > 7 {
						// make char uppercase if ith character is 9..f
						acc.push_str(&address_char.to_uppercase().to_string())
					} else {
						// already lowercased
						acc.push(address_char)
					}

					acc
				});
		write!(f, "{checksum}")
	}
}

#[cfg(feature = "serde")]
impl core::str::FromStr for Address {
	type Err = &'static str;

	// NOTE: For strict conversion, we should check the checksum.
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let data: [u8; 20] = const_hex::decode_to_array(s).map_err(|_| "invalid address")?;
		Ok(Address(data))
	}
}

impl core::fmt::Debug for Address {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "{}", sp_core::hexdisplay::HexDisplay::from(&self.0))
	}
}

#[cfg(feature = "serde")]
impl Serialize for Address {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		use alloc::string::ToString;
		serializer.serialize_str(&self.to_string())
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Address {
	fn deserialize<D>(deserializer: D) -> Result<Address, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use core::str::FromStr;
		let s = String::deserialize(deserializer)?;
		Address::from_str(&s).map_err(serde::de::Error::custom)
	}
}

impl<AccountId: From<H256>> AccountIdConversion<AccountId> for Address {
	fn into_account_truncating(&self) -> AccountId {
		let mut data = [0u8; 24];
		data[0..4].copy_from_slice(b"evm:");
		data[4..24].copy_from_slice(&self.0);
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
	fn display_ethereum_address() {
		let address: Address = dev_public().into();
		assert_eq!(address.to_string(), "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac");
		assert_eq!(format!("{:?}", address), "f24ff3a9cf04c71dbc94d0b566f7a27b94566cac");
	}
}
