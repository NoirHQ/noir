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

//! Noir primitive types for Nostr compatibility.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "serde")]
use alloc::string::String;
#[cfg(feature = "serde")]
use bech32::{Bech32, Hrp};
use buidl::FixedBytes;
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sp_core::{ecdsa, H256};
use sp_runtime::traits::AccountIdConversion;

#[cfg(feature = "serde")]
const NPUB: Hrp = Hrp::parse_unchecked("npub");

/// Nostr address.
#[derive(FixedBytes)]
#[buidl(derive(Substrate), skip_derive(PassBy))]
pub struct Address([u8; 32]);

impl From<H256> for Address {
	fn from(h: H256) -> Self {
		Self(h.0)
	}
}

impl From<Address> for H256 {
	fn from(v: Address) -> Self {
		Self(v.0)
	}
}

impl From<ecdsa::Public> for Address {
	fn from(key: ecdsa::Public) -> Self {
		Self(key.0[1..].try_into().unwrap())
	}
}

#[cfg(feature = "serde")]
impl core::fmt::Display for Address {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "{}", bech32::encode::<Bech32>(NPUB, &self.0).expect("bech32 encode"))
	}
}

#[cfg(feature = "serde")]
impl core::str::FromStr for Address {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let (hrp, data) = bech32::decode(s).map_err(|_| "bech32 decode")?;
		if hrp != NPUB {
			return Err("invalid bech32 prefix");
		}
		let data: [u8; 32] = data.try_into().map_err(|_| "invalid data length")?;
		Ok(Self(data))
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
		H256::from(self.clone()).into()
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
	fn display_nostr_address() {
		let address: Address = dev_public().into();
		assert_eq!(
			address.to_string(),
			"npub12z25pyvl4t8e4dfpgmy65sxmdqtjmqmhwfgt9rjx0ytkujwvmk0s2yfk08"
		);
	}

	#[test]
	fn parse_nostr_address() {
		use std::str::FromStr;

		let address: Address = dev_public().into();
		let parsed =
			Address::from_str("npub12z25pyvl4t8e4dfpgmy65sxmdqtjmqmhwfgt9rjx0ytkujwvmk0s2yfk08")
				.expect("parse nostr address");
		assert_eq!(address, parsed);
	}
}
