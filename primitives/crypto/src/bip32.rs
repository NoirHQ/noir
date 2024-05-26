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
use hmac::{Hmac, Mac};
#[cfg(feature = "full_crypto")]
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "full_crypto")]
use sha2::Sha512;
#[cfg(feature = "full_crypto")]
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "std")]
use regex::Regex;
#[cfg(feature = "std")]
use sp_core::crypto::SecretStringError;

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

	pub fn unwrap(self) -> [u8; 4] {
		match self {
			Self::Soft(inner) | Self::Hard(inner) => inner,
		}
	}

	pub fn harden(self) -> Self {
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
pub trait Curve: Clone {
	fn secret(secret: &[u8]) -> Result<[u8; 32], ()>;
	fn public(secret: &[u8]) -> Result<[u8; 33], ()>;
	fn scalar_add(v: &[u8], w: &[u8]) -> Result<[u8; 32], ()>;
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
	pub fn new(secret: [u8; 32], chain_code: [u8; 32]) -> Result<Self, ()> {
		C::secret(&secret[..])?;
		Ok(Self { secret, chain_code, marker: Default::default() })
	}

	pub fn from_seed(seed: &[u8]) -> Result<Self, ()> {
		let mut mac = Hmac::<Sha512>::new_from_slice(b"Bitcoin seed").unwrap();
		mac.update(seed);

		let result = mac.finalize().into_bytes();

		let mut secret = [0u8; 32];
		let mut chain_code = [0u8; 32];

		secret.copy_from_slice(&result[0..32]);
		chain_code.copy_from_slice(&result[32..]);

		Self::new(secret, chain_code)
	}

	#[cfg(feature = "std")]
	pub fn from_phrase(phrase: &str, password: Option<&str>) -> Result<Self, ()> {
		use bip39::{Language, Mnemonic, Seed};
		let mnemonic = Mnemonic::from_phrase(phrase, Language::English).map_err(|_| ())?;
		let seed = Seed::new(&mnemonic, password.unwrap_or(""));
		Self::from_seed(seed.as_bytes())
	}

	pub fn public(&self) -> [u8; 33] {
		C::public(self.as_ref()).unwrap()
	}

	pub fn scalar_add(v: &[u8], w: &[u8]) -> Result<[u8; 32], ()> {
		C::scalar_add(v, w)
	}

	pub fn derive<Iter: Iterator<Item = DeriveJunction>>(&self, path: Iter) -> Result<Self, ()> {
		let mut ext = self.clone();
		for i in path {
			ext = ext.child(i)?;
		}

		Ok(ext)
	}

	pub fn child(&self, i: DeriveJunction) -> Result<Self, ()> {
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

		secret = Self::scalar_add(&secret, self.as_ref())?;

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
pub mod secp256k1 {
	use secp256k1::{Scalar, SecretKey};

	#[cfg(all(feature = "full_crypto", not(feature = "std")))]
	use secp256k1::Secp256k1;
	#[cfg(feature = "std")]
	use secp256k1::SECP256K1;

	pub type ExtendedPrivateKey = super::ExtendedPrivateKey<Curve>;

	#[derive(Clone)]
	pub struct Curve;

	impl super::Curve for Curve {
		fn secret(secret: &[u8]) -> Result<[u8; 32], ()> {
			Ok(SecretKey::from_slice(secret).map_err(|_| ())?.secret_bytes())
		}

		fn public(secret: &[u8]) -> Result<[u8; 33], ()> {
			let s = SecretKey::from_slice(secret).map_err(|_| ())?;

			#[cfg(feature = "std")]
			let context = SECP256K1;
			#[cfg(not(feature = "std"))]
			let context = Secp256k1::signing_only();

			Ok(s.public_key(context).serialize())
		}

		fn scalar_add(v: &[u8], w: &[u8]) -> Result<[u8; 32], ()> {
			let v = SecretKey::from_slice(v).map_err(|_| ())?;
			let w = <[u8; 32]>::try_from(w).map_err(|_| ())?;
			let w = Scalar::from_be_bytes(w).map_err(|_| ())?;
			Ok(v.add_tweak(&w).map_err(|_| ())?.secret_bytes())
		}
	}
}

#[cfg(feature = "full_crypto")]
pub mod secp256r1 {
	use p256::{
		elliptic_curve::{ops::Add, sec1::ToEncodedPoint, ScalarPrimitive},
		NistP256, SecretKey,
	};

	pub type ExtendedPrivateKey = super::ExtendedPrivateKey<Curve>;

	#[derive(Clone)]
	pub struct Curve;

	impl super::Curve for Curve {
		fn secret(secret: &[u8]) -> Result<[u8; 32], ()> {
			SecretKey::from_slice(secret).map_err(|_| ())?;
			<[u8; 32]>::try_from(secret).map_err(|_| ())
		}

		fn public(secret: &[u8]) -> Result<[u8; 33], ()> {
			let s = SecretKey::from_slice(secret).map_err(|_| ())?;
			let p = s.public_key().to_encoded_point(true);
			let mut x = [0u8; 33];
			x.copy_from_slice(p.as_bytes());
			Ok(x)
		}

		fn scalar_add(v: &[u8], w: &[u8]) -> Result<[u8; 32], ()> {
			let v = ScalarPrimitive::<NistP256>::from_slice(v).unwrap();
			let w = ScalarPrimitive::<NistP256>::from_slice(w).unwrap();
			let mut x = [0u8; 32];
			x.copy_from_slice(&v.add(&w).to_bytes()[..]);
			Ok(x)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{secp256k1::ExtendedPrivateKey, DeriveJunction};
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
		let junctions = DeriveJunction::parse(DEV_PATH).unwrap();

		let a = ExtendedPrivateKey::from_phrase(DEV_PHRASE, None);
		assert!(a.is_ok());
		let a = a.unwrap().derive(junctions.into_iter());
		assert!(a.is_ok());
		assert_eq!(
			array_bytes::bytes2hex("", a.unwrap().as_ref()),
			"5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133"
		);
	}
}
