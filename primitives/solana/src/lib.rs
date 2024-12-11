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

//! Noir primitive types for Solana compatibility.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod account;
pub mod clock;
pub mod commitment_config;
pub mod config;
pub mod epoch_info;
pub mod instruction_error;
pub mod response;
pub mod transaction_error;

#[cfg(feature = "serde")]
use alloc::string::String;
use buidl::FixedBytes;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sp_core::{ed25519, H256};

/// Solana address.
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

impl From<ed25519::Public> for Address {
	fn from(key: ed25519::Public) -> Self {
		Address(key.0)
	}
}

#[cfg(feature = "serde")]
impl core::fmt::Display for Address {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "{}", bs58::encode(&self.0).into_string())
	}
}

#[cfg(feature = "serde")]
impl core::str::FromStr for Address {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Address(bs58::decode(s.as_bytes()).into_array_const().map_err(|_| "invalid address")?))
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

#[cfg(test)]
mod tests {
	use super::*;
	use sp_core::{ed25519, Pair};

	fn dev_public() -> ed25519::Public {
		ed25519::Pair::from_string("//Alice", None).unwrap().public()
	}

	#[test]
	fn display_solana_address() {
		let alice = "ADFCNGW3av5BR6Jm5mvjEfdTGqcsfFQWPEvkB47AHHcq";
		assert_eq!(Address::from(dev_public()).to_string(), alice);
	}
}
