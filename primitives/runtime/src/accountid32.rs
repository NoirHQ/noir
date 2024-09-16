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

use crate::traits::{Checkable, Property};
#[cfg(feature = "serde")]
use alloc::{format, string::String};
use buidl::FixedBytes;
use scale_info::{Type, TypeInfo};
#[cfg(feature = "serde")]
use sp_core::crypto::Ss58Codec;
use sp_core::{crypto, ecdsa, ed25519, sr25519, H256};
use sp_io::hashing::blake2_256;

/// An opaque 32-byte cryptographic identifier.
#[derive(FixedBytes)]
#[buidl(substrate(Codec, Core))]
pub struct AccountId32<T: Clone>([u8; 32], Option<T>);

impl<T: Clone> AccountId32<T> {
	pub const fn new(inner: [u8; 32]) -> Self {
		Self(inner, None)
	}
}

impl<T: Clone> TypeInfo for AccountId32<T> {
	type Identity = crypto::AccountId32;

	fn type_info() -> Type {
		Self::Identity::type_info()
	}
}

#[cfg(feature = "serde")]
impl<T: Clone> Ss58Codec for AccountId32<T> {}

impl<T: Clone> From<H256> for AccountId32<T> {
	fn from(h: H256) -> Self {
		Self(h.0, None)
	}
}

impl<T: Clone> From<crypto::AccountId32> for AccountId32<T> {
	fn from(acc: crypto::AccountId32) -> Self {
		Self(Into::<[u8; 32]>::into(acc), None)
	}
}

impl<T: Clone> From<AccountId32<T>> for crypto::AccountId32 {
	fn from(acc: AccountId32<T>) -> Self {
		Self::from(acc.0)
	}
}

#[cfg(feature = "std")]
impl<T: Clone> std::fmt::Display for AccountId32<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.to_ss58check())
	}
}

impl<T: Clone> core::fmt::Debug for AccountId32<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		#[cfg(feature = "serde")]
		{
			let s = self.to_ss58check();
			write!(f, "{} ({}...)", sp_core::hexdisplay::HexDisplay::from(&self.0), &s[0..8])?;
		}

		#[cfg(not(feature = "serde"))]
		write!(f, "{}", sp_core::hexdisplay::HexDisplay::from(&self.0))?;

		Ok(())
	}
}

#[cfg(feature = "serde")]
impl<T: Clone> serde::Serialize for AccountId32<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.to_ss58check())
	}
}

#[cfg(feature = "serde")]
impl<'de, T: Clone> serde::Deserialize<'de> for AccountId32<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ss58Codec::from_ss58check(&String::deserialize(deserializer)?)
			.map_err(|e| serde::de::Error::custom(format!("{:?}", e)))
	}
}

#[cfg(feature = "std")]
impl<T: Clone> std::str::FromStr for AccountId32<T> {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let hex_or_ss58_without_prefix = s.trim_start_matches("0x");
		if hex_or_ss58_without_prefix.len() == 64 {
			const_hex::decode_to_array(hex_or_ss58_without_prefix)
				.map(Into::into)
				.map_err(|_| "invalid hex address.")
		} else {
			Self::from_ss58check(s).map_err(|_| "invalid ss58 address.")
		}
	}
}

impl<T: Clone> From<ed25519::Public> for AccountId32<T>
where
	T: From<ed25519::Public>,
{
	fn from(key: ed25519::Public) -> Self {
		Self(key.0, Some(key.into()))
	}
}

impl<T: Clone> From<sr25519::Public> for AccountId32<T>
where
	T: From<sr25519::Public>,
{
	fn from(key: sr25519::Public) -> Self {
		Self(key.0, Some(key.into()))
	}
}

impl<T: Clone> From<ecdsa::Public> for AccountId32<T>
where
	T: From<ecdsa::Public>,
{
	fn from(key: ecdsa::Public) -> Self {
		Self(blake2_256(&key.0), Some(key.into()))
	}
}

impl<T: Clone, E> TryFrom<AccountId32<T>> for ecdsa::Public
where
	T: TryInto<ecdsa::Public, Error = E>,
	E: Default,
{
	type Error = E;

	fn try_from(account: AccountId32<T>) -> Result<Self, Self::Error> {
		match account.1 {
			Some(key) => key.try_into(),
			None => Err(Default::default()),
		}
	}
}

impl<T: Clone> Property for AccountId32<T> {
	type Value = Option<T>;

	fn get(&self) -> &Option<T> {
		&self.1
	}

	fn set(&mut self, value: Option<T>) {
		self.1 = value;
	}
}

impl<T: Clone> Checkable<ed25519::Public> for AccountId32<T>
where
	T: From<ed25519::Public>,
{
	type Output = bool;

	fn check(&mut self, value: ed25519::Public) -> Self::Output {
		(self.0 == value.0)
			.then(|| {
				self.1 = Some(value.into());
			})
			.is_some()
	}
}

impl<T: Clone> Checkable<sr25519::Public> for AccountId32<T>
where
	T: From<sr25519::Public>,
{
	type Output = bool;

	fn check(&mut self, value: sr25519::Public) -> Self::Output {
		(self.0 == value.0)
			.then(|| {
				self.1 = Some(value.into());
			})
			.is_some()
	}
}

impl<T: Clone> Checkable<ecdsa::Public> for AccountId32<T>
where
	T: From<ecdsa::Public>,
{
	type Output = bool;

	fn check(&mut self, value: ecdsa::Public) -> Self::Output {
		(self.0 == blake2_256(&value))
			.then(|| {
				self.1 = Some(value.into());
			})
			.is_some()
	}
}

#[cfg(test)]
mod tests {
	type AccountId32 = super::AccountId32<crate::MultiSigner>;

	#[test]
	fn accountid_32_from_str_works() {
		use std::str::FromStr;
		assert!(AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").is_ok());
		assert!(AccountId32::from_str(
			"5c55177d67b064bb5d189a3e1ddad9bc6646e02e64d6e308f5acbb1533ac430d"
		)
		.is_ok());
		assert!(AccountId32::from_str(
			"0x5c55177d67b064bb5d189a3e1ddad9bc6646e02e64d6e308f5acbb1533ac430d"
		)
		.is_ok());

		assert_eq!(
			AccountId32::from_str("99G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap_err(),
			"invalid ss58 address.",
		);
		assert_eq!(
			AccountId32::from_str(
				"gc55177d67b064bb5d189a3e1ddad9bc6646e02e64d6e308f5acbb1533ac430d"
			)
			.unwrap_err(),
			"invalid hex address.",
		);
		assert_eq!(
			AccountId32::from_str(
				"0xgc55177d67b064bb5d189a3e1ddad9bc6646e02e64d6e308f5acbb1533ac430d"
			)
			.unwrap_err(),
			"invalid hex address.",
		);

		// valid hex but invalid length will be treated as ss58.
		assert_eq!(
			AccountId32::from_str(
				"55c55177d67b064bb5d189a3e1ddad9bc6646e02e64d6e308f5acbb1533ac430d"
			)
			.unwrap_err(),
			"invalid ss58 address.",
		);
	}
}
