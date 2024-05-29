// This file is part of Noir.

// Copyright (C) 2023 Haderech Pte. Ltd.
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

use crate::{Derived, UniversalAddress};
use np_crypto::ecdsa::EcdsaExt;
use parity_scale_codec::{Decode, Encode, EncodeLike, Error, Input, MaxEncodedLen};
use scale_info::{Type, TypeInfo};
#[cfg(feature = "serde")]
use sp_core::crypto::{PublicError, Ss58Codec};
use sp_core::{
	crypto::{AccountId32 as SubstrateAccountId32, FromEntropy, UncheckedFrom},
	ByteArray, H160, H256,
};
#[cfg(all(feature = "serde", not(feature = "std")))]
use sp_std::{
	alloc::{format, string::String},
	vec::Vec,
};

#[derive(Clone, Eq)]
pub struct AccountId32 {
	inner: [u8; 32],
	pub origin: Option<UniversalAddress>,
}

impl AccountId32 {
	pub const fn new(inner: [u8; 32]) -> Self {
		Self { inner, origin: None }
	}
}

impl Derived for AccountId32 {
	type Origin = UniversalAddress;

	fn origin(&self) -> Option<Self::Origin> {
		self.origin.clone()
	}
}

impl PartialEq for AccountId32 {
	fn eq(&self, other: &Self) -> bool {
		self.inner.eq(&other.inner)
	}
}

impl Ord for AccountId32 {
	fn cmp(&self, other: &Self) -> sp_std::cmp::Ordering {
		self.inner.cmp(&other.inner)
	}
}

impl PartialOrd for AccountId32 {
	fn partial_cmp(&self, other: &Self) -> Option<sp_std::cmp::Ordering> {
		self.inner.partial_cmp(&other.inner)
	}
}

impl Encode for AccountId32 {
	fn size_hint(&self) -> usize {
		self.inner.size_hint()
	}

	fn encode(&self) -> Vec<u8> {
		self.inner.encode()
	}
}

impl EncodeLike for AccountId32 {}

impl Decode for AccountId32 {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(Self { inner: <[u8; 32]>::decode(input)?, origin: None })
	}
}

impl MaxEncodedLen for AccountId32 {
	fn max_encoded_len() -> usize {
		32
	}
}

impl TypeInfo for AccountId32 {
	type Identity = <SubstrateAccountId32 as TypeInfo>::Identity;

	fn type_info() -> Type {
		<SubstrateAccountId32 as TypeInfo>::type_info()
	}
}

#[cfg(feature = "std")]
impl sp_std::hash::Hash for AccountId32 {
	fn hash<H: sp_std::hash::Hasher>(&self, state: &mut H) {
		self.inner.hash(state);
	}
}

impl UncheckedFrom<H256> for AccountId32 {
	fn unchecked_from(h: H256) -> Self {
		Self { inner: h.into(), origin: None }
	}
}

impl ByteArray for AccountId32 {
	const LEN: usize = 32;
}

#[cfg(feature = "serde")]
impl Ss58Codec for AccountId32 {
	fn to_ss58check(&self) -> String {
		SubstrateAccountId32::new(self.inner).to_ss58check()
	}

	fn from_ss58check(s: &str) -> Result<Self, PublicError> {
		Ok(Self { inner: SubstrateAccountId32::from_ss58check(s)?.into(), origin: None })
	}
}

impl AsRef<[u8]> for AccountId32 {
	fn as_ref(&self) -> &[u8] {
		&self.inner
	}
}

impl AsMut<[u8]> for AccountId32 {
	fn as_mut(&mut self) -> &mut [u8] {
		&mut self.inner
	}
}

impl AsRef<[u8; 32]> for AccountId32 {
	fn as_ref(&self) -> &[u8; 32] {
		&self.inner
	}
}

impl AsMut<[u8; 32]> for AccountId32 {
	fn as_mut(&mut self) -> &mut [u8; 32] {
		&mut self.inner
	}
}

impl From<[u8; 32]> for AccountId32 {
	fn from(v: [u8; 32]) -> Self {
		Self::new(v)
	}
}

impl<'a> TryFrom<&'a [u8]> for AccountId32 {
	type Error = ();

	fn try_from(v: &'a [u8]) -> Result<Self, Self::Error> {
		if v.len() == 32 {
			let mut inner = [0u8; 32];
			inner.copy_from_slice(v);
			Ok(Self::new(inner))
		} else {
			Err(())
		}
	}
}

impl From<H256> for AccountId32 {
	fn from(v: H256) -> Self {
		Self::new(v.into())
	}
}

impl From<sp_core::ecdsa::Public> for AccountId32 {
	fn from(v: sp_core::ecdsa::Public) -> Self {
		Self { inner: sp_core::blake2_256(v.as_ref()), origin: Some(v.into()) }
	}
}

impl From<AccountId32> for [u8; 32] {
	fn from(v: AccountId32) -> [u8; 32] {
		v.inner
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for AccountId32 {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.to_ss58check())
	}
}

impl sp_std::fmt::Debug for AccountId32 {
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		#[cfg(feature = "serde")]
		{
			let s = self.to_ss58check();
			write!(f, "{} ({}...)", sp_core::hexdisplay::HexDisplay::from(&self.inner), &s[0..8])?;
		}

		#[cfg(not(feature = "serde"))]
		write!(f, "{}", sp_core::hexdisplay::HexDisplay::from(&self.0))?;

		Ok(())
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for AccountId32 {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.to_ss58check())
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for AccountId32 {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ss58Codec::from_ss58check(&String::deserialize(deserializer)?)
			.map_err(|e| serde::de::Error::custom(format!("{:?}", e)))
	}
}

#[cfg(feature = "std")]
impl sp_std::str::FromStr for AccountId32 {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let hex_or_ss58_without_prefix = s.trim_start_matches("0x");
		if hex_or_ss58_without_prefix.len() == 64 {
			array_bytes::hex_n_into(hex_or_ss58_without_prefix).map_err(|_| "invalid hex address.")
		} else {
			Self::from_ss58check(s).map_err(|_| "invalid ss58 address.")
		}
	}
}

/// Creates an [`AccountId32`] from the input, which should contain at least 32 bytes.
impl FromEntropy for AccountId32 {
	fn from_entropy(
		input: &mut impl parity_scale_codec::Input,
	) -> Result<Self, parity_scale_codec::Error> {
		Ok(AccountId32::new(FromEntropy::from_entropy(input)?))
	}
}

impl EcdsaExt for AccountId32 {
	fn to_eth_address(&self) -> Option<H160> {
		self.origin.as_ref().and_then(EcdsaExt::to_eth_address)
	}

	fn to_cosm_address(&self) -> Option<H160> {
		self.origin.as_ref().and_then(EcdsaExt::to_cosm_address)
	}
}
