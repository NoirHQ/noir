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

//! BIP32 key derivation.

// TODO: add documentation
#![doc(hidden)]

#[cfg(feature = "full_crypto")]
pub mod k1;
#[cfg(feature = "full_crypto")]
pub mod r1;

use codec::{Decode, Encode};

#[cfg(feature = "full_crypto")]
use hmac::{Hmac, Mac};
#[cfg(feature = "full_crypto")]
use sha2::Sha512;
#[cfg(feature = "full_crypto")]
use sp_core::crypto::SecretStringError;

#[cfg(feature = "std")]
use regex::Regex;

#[cfg(feature = "std")]
lazy_static::lazy_static! {
	static ref PATH_REGEX: Regex = Regex::new(r"^m(?P<path>(/\d+'?)*)$")
		.expect("constructed from known-good static value; qed");
	static ref JUNCTION_REGEX: Regex = Regex::new(r"/([^/']+'?)")
		.expect("constructed from known-good static value; qed");
}

#[cfg(feature = "full_crypto")]
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub enum DeriveJunction {
	Soft([u8; 4]),
	Hard([u8; 4]),
}

#[cfg(feature = "full_crypto")]
impl DeriveJunction {
	const HARDENED_MODIFIER: u32 = 0x80000000u32;

	#[cfg(feature = "std")]
	pub fn parse(path: &str) -> Result<Vec<Self>, SecretStringError> {
		let cap = PATH_REGEX.captures(path).ok_or(SecretStringError::InvalidPath)?;

		JUNCTION_REGEX.captures_iter(&cap["path"]).try_fold(vec![], |mut junctions, j| {
			match Self::try_from(&j[1]) {
				Ok(j) => {
					junctions.push(j);
					Ok(junctions)
				},
				Err(_) => Err(SecretStringError::InvalidPath),
			}
		})
	}

	fn unwrap(self) -> [u8; 4] {
		match self {
			Self::Soft(inner) | Self::Hard(inner) => inner,
		}
	}

	fn harden(self) -> Self {
		let mut inner = self.unwrap();
		inner[0] |= 0x80u8;
		Self::Hard(inner)
	}
}

#[cfg(feature = "full_crypto")]
impl AsRef<[u8]> for DeriveJunction {
	fn as_ref(&self) -> &[u8] {
		match self {
			Self::Soft(ref x) | Self::Hard(ref x) => x,
		}
	}
}

#[cfg(feature = "full_crypto")]
impl From<u32> for DeriveJunction {
	fn from(index: u32) -> Self {
		if index >= Self::HARDENED_MODIFIER {
			Self::Hard(index.to_be_bytes())
		} else {
			Self::Soft(index.to_be_bytes())
		}
	}
}

#[cfg(feature = "full_crypto")]
impl TryFrom<&str> for DeriveJunction {
	type Error = ();

	fn try_from(index_str: &str) -> Result<Self, ()> {
		let (code, hard) = if let Some(stripped) = index_str.strip_suffix("'") {
			(stripped, true)
		} else {
			(index_str, false)
		};

		let mut index = str::parse::<u32>(code).map_err(|_| ())?;
		if hard {
			if index < Self::HARDENED_MODIFIER {
				index += Self::HARDENED_MODIFIER;
			} else {
				return Err(())
			}
		}
		Ok(Self::from(index))
	}
}

#[cfg(feature = "full_crypto")]
pub trait ExtendedPrivateKey: Sized + zeroize::ZeroizeOnDrop {
	fn new(secret: [u8; 32], chain_code: [u8; 32]) -> Self;

	fn secret(&self) -> &[u8];

	fn chain_code(&self) -> &[u8];

	fn public(secret: &[u8]) -> Result<[u8; 33], ()>;

	fn scalar_add(v: &[u8], w: &[u8]) -> [u8; 32];

	fn derive<Iter: Iterator<Item = DeriveJunction>>(seed: &[u8], path: Iter) -> Result<Self, ()> {
		let mut mac = Hmac::<Sha512>::new_from_slice(b"Bitcoin seed").unwrap();
		mac.update(seed);

		let result = mac.finalize().into_bytes();

		let mut secret = [0u8; 32];
		let mut chain_code = [0u8; 32];

		secret.copy_from_slice(&result[0..32]);
		chain_code.copy_from_slice(&result[32..]);

		let mut ext: Self = ExtendedPrivateKey::new(secret, chain_code);
		for i in path {
			ext = ext.child(i)?;
		}

		Ok(ext)
	}

	fn child(&mut self, i: DeriveJunction) -> Result<Self, ()> {
		let mut mac = Hmac::<Sha512>::new_from_slice(self.chain_code()).unwrap();

		match i {
			DeriveJunction::Soft(_) => {
				let pubkey = Self::public(self.secret())?;
				mac.update(&pubkey);
			},
			DeriveJunction::Hard(_) => {
				mac.update(&[0u8; 1]);
				mac.update(self.secret());
			},
		};
		mac.update(i.as_ref());

		let result = mac.finalize().into_bytes();

		let mut secret = [0u8; 32];
		let mut chain_code = [0u8; 32];

		secret.copy_from_slice(&result[0..32]);
		chain_code.copy_from_slice(&result[32..]);

		secret = Self::scalar_add(&secret, self.secret());

		Ok(ExtendedPrivateKey::new(secret, chain_code))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const DEV_PATH: &str = "m/44'/60'/0'/0/0";

	#[test]
	fn parse_derivation_path() {
		let junctions = vec![
			DeriveJunction::from(44).harden(),
			DeriveJunction::from(60).harden(),
			DeriveJunction::from(0).harden(),
			DeriveJunction::from(0),
			DeriveJunction::from(0),
		];
		let a = DeriveJunction::parse(DEV_PATH);
		assert!(a.is_ok());
		assert_eq!(a.unwrap(), junctions);
	}
}
