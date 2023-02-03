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
use codec::{Decode, Encode};
#[cfg(feature = "full_crypto")]
use hmac::{Hmac, Mac};
#[cfg(feature = "full_crypto")]
use sha2::Sha512;
#[cfg(feature = "full_crypto")]
use sp_core::crypto::SecretStringError;
#[cfg(feature = "full_crypto")]
use zeroize::{Zeroize, ZeroizeOnDrop};

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
pub trait Curve {
	fn secret(secret: &[u8]) -> Result<[u8; 32], ()>;
	fn public(secret: &[u8]) -> Result<[u8; 33], ()>;
	fn scalar_add(v: &[u8], w: &[u8]) -> [u8; 32];
}

#[cfg(feature = "full_crypto")]
#[derive(Clone, PartialEq, Eq, Debug, Zeroize, ZeroizeOnDrop)]
pub struct ExtendedPrivateKey<C: Curve> {
	secret: [u8; 32],
	chain_code: [u8; 32],
	marker: sp_std::marker::PhantomData<C>,
}

#[cfg(feature = "full_crypto")]
impl<C: Curve> ExtendedPrivateKey<C> {
	fn new(secret: [u8; 32], chain_code: [u8; 32]) -> Result<Self, ()> {
		C::secret(&secret[..])?;
		Ok(Self { secret, chain_code, marker: Default::default() })
	}

	fn from_seed(seed: &[u8]) -> Result<Self, ()> {
		let mut mac = Hmac::<Sha512>::new_from_slice(b"Bitcoin seed").unwrap();
		mac.update(seed);

		let result = mac.finalize().into_bytes();

		let mut secret = [0u8; 32];
		let mut chain_code = [0u8; 32];

		secret.copy_from_slice(&result[0..32]);
		chain_code.copy_from_slice(&result[32..]);

		Self::new(secret, chain_code)
	}

	fn public(&self) -> [u8; 33] {
		C::public(self.as_ref()).unwrap()
	}

	fn scalar_add(v: &[u8], w: &[u8]) -> [u8; 32] {
		C::scalar_add(v, w)
	}

	fn derive<Iter: Iterator<Item = DeriveJunction>>(seed: &[u8], path: Iter) -> Result<Self, ()> {
		let mut ext = Self::from_seed(seed)?;
		for i in path {
			ext = ext.child(i)?;
		}

		Ok(ext)
	}

	fn child(&mut self, i: DeriveJunction) -> Result<Self, ()> {
		let mut mac = Hmac::<Sha512>::new_from_slice(&self.chain_code[..]).unwrap();

		match i {
			DeriveJunction::Soft(_) => {
				let pubkey = self.public();
				mac.update(&pubkey);
			},
			DeriveJunction::Hard(_) => {
				mac.update(&[0u8; 1]);
				mac.update(self.as_ref());
			},
		};
		mac.update(i.as_ref());

		let result = mac.finalize().into_bytes();

		let mut secret = [0u8; 32];
		let mut chain_code = [0u8; 32];

		secret.copy_from_slice(&result[0..32]);
		chain_code.copy_from_slice(&result[32..]);

		secret = Self::scalar_add(&secret, self.as_ref());

		Self::new(secret, chain_code)
	}
}

#[cfg(feature = "full_crypto")]
impl<C: Curve> AsRef<[u8]> for ExtendedPrivateKey<C> {
	fn as_ref(&self) -> &[u8] {
		&self.secret[..]
	}
}

#[cfg(feature = "full_crypto")]
mod secp256k1 {
	use k256::{
		elliptic_curve::{ops::Add, sec1::ToEncodedPoint, ScalarCore},
		Secp256k1, SecretKey,
	};

	pub type ExtendedPrivateKey = super::ExtendedPrivateKey<Curve>;

	pub struct Curve;

	impl super::Curve for Curve {
		fn secret(secret: &[u8]) -> Result<[u8; 32], ()> {
			SecretKey::from_be_bytes(secret).map_err(|_| ())?;
			<[u8; 32]>::try_from(secret).map_err(|_| ())
		}

		fn public(secret: &[u8]) -> Result<[u8; 33], ()> {
			let s = SecretKey::from_be_bytes(secret).map_err(|_| ())?;
			let p = s.public_key().to_encoded_point(true);
			let mut x = [0u8; 33];
			x.copy_from_slice(p.as_bytes());
			Ok(x)
		}

		fn scalar_add(v: &[u8], w: &[u8]) -> [u8; 32] {
			let v = ScalarCore::<Secp256k1>::from_be_slice(v).unwrap();
			let w = ScalarCore::<Secp256k1>::from_be_slice(w).unwrap();
			let mut x = [0u8; 32];
			x.copy_from_slice(&v.add(&w).to_be_bytes()[..]);
			x
		}
	}
}

#[cfg(feature = "full_crypto")]
mod secp256r1 {
	use p256::{
		elliptic_curve::{ops::Add, sec1::ToEncodedPoint, ScalarCore},
		NistP256, SecretKey,
	};

	pub type ExtendedPrivateKey = super::ExtendedPrivateKey<Curve>;

	pub struct Curve;

	impl super::Curve for Curve {
		fn secret(secret: &[u8]) -> Result<[u8; 32], ()> {
			SecretKey::from_be_bytes(secret).map_err(|_| ())?;
			<[u8; 32]>::try_from(secret).map_err(|_| ())
		}

		fn public(secret: &[u8]) -> Result<[u8; 33], ()> {
			let s = SecretKey::from_be_bytes(secret).map_err(|_| ())?;
			let p = s.public_key().to_encoded_point(true);
			let mut x = [0u8; 33];
			x.copy_from_slice(p.as_bytes());
			Ok(x)
		}

		fn scalar_add(v: &[u8], w: &[u8]) -> [u8; 32] {
			let v = ScalarCore::<NistP256>::from_be_slice(v).unwrap();
			let w = ScalarCore::<NistP256>::from_be_slice(w).unwrap();
			let mut x = [0u8; 32];
			x.copy_from_slice(&v.add(&w).to_be_bytes()[..]);
			x
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{secp256k1::ExtendedPrivateKey, DeriveJunction};
	use bip39::{Language, Mnemonic, Seed};
	use sp_core::crypto::DEV_PHRASE;

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

	#[test]
	fn derive_from_dev_phrase() {
		let mnemonic = Mnemonic::from_phrase(DEV_PHRASE, Language::English).unwrap();
		let seed = Seed::new(&mnemonic, "");
		let junctions = DeriveJunction::parse(DEV_PATH).unwrap();

		let a = ExtendedPrivateKey::derive(seed.as_bytes(), junctions.into_iter());
		assert!(a.is_ok());
		assert_eq!(
			array_bytes::bytes2hex("", a.unwrap().as_ref()),
			"5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133"
		);
	}
}
