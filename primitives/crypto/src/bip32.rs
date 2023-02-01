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
use sp_core::crypto::{SecretStringError, DEV_PHRASE};

#[cfg(feature = "std")]
use regex::Regex;
#[cfg(feature = "std")]
use secrecy::SecretString;

#[cfg(feature = "full_crypto")]
pub const JUNCTION_ID_LEN: usize = 4;

#[cfg(feature = "std")]
lazy_static::lazy_static! {
	static ref SECRET_PHRASE_REGEX: Regex = Regex::new(r"^(((?P<phrase>[\d\w ]+)/m)?|m)(?P<path>(/[^/']+'?)*)(///(?P<password>.*))?$")
		.expect("constructed from known-good static value; qed");
	static ref JUNCTION_REGEX: Regex = Regex::new(r"/([^/']+'?)")
		.expect("constructed from known-good static value; qed");
}

#[cfg(feature = "std")]
pub struct SecretUri {
	pub phrase: SecretString,
	pub password: Option<SecretString>,
	pub junctions: Vec<DeriveJunction>,
}

impl sp_std::str::FromStr for SecretUri {
	type Err = SecretStringError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let cap = SECRET_PHRASE_REGEX.captures(s).ok_or(SecretStringError::InvalidFormat)?;

		let junctions = JUNCTION_REGEX
			.captures_iter(&cap["path"])
			.map(|f| DeriveJunction::from(&f[1]))
			.collect::<Vec<_>>();

		let phrase = cap.name("phrase").map(|r| r.as_str()).unwrap_or(DEV_PHRASE);
		let password = cap.name("password");

		Ok(Self {
			phrase: SecretString::from_str(phrase).expect("Returns infallible error; qed"),
			password: password.map(|v| {
				SecretString::from_str(v.as_str()).expect("Returns infallible error; qed")
			}),
			junctions,
		})
	}
}

#[cfg(feature = "full_crypto")]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub enum DeriveJunction {
	Soft([u8; JUNCTION_ID_LEN]),
	Hard([u8; JUNCTION_ID_LEN]),
}

#[cfg(feature = "full_crypto")]
impl DeriveJunction {
	pub fn soften(self) -> Self {
		let mut inner = self.unwrap_inner();
		inner[0] &= !(0x80u8);
		DeriveJunction::Soft(inner)
	}

	pub fn harden(self) -> Self {
		let mut inner = self.unwrap_inner();
		inner[0] |= 0x80u8;
		DeriveJunction::Hard(inner)
	}

	pub fn soft<T: Encode>(index: T) -> Self {
		let mut cc: [u8; JUNCTION_ID_LEN] = Default::default();
		index.using_encoded(|data| {
			if data.len() > JUNCTION_ID_LEN {
				cc.copy_from_slice(&sp_core::hashing::blake2_256(data)[0..JUNCTION_ID_LEN]);
			} else {
				cc[0..data.len()].copy_from_slice(data);
			}
		});
		DeriveJunction::Soft(cc)
	}

	pub fn hard<T: Encode>(index: T) -> Self {
		Self::soft(index).harden()
	}

	pub fn unwrap_inner(self) -> [u8; JUNCTION_ID_LEN] {
		match self {
			DeriveJunction::Hard(c) | DeriveJunction::Soft(c) => c,
		}
	}

	pub fn inner(&self) -> &[u8; JUNCTION_ID_LEN] {
		match self {
			DeriveJunction::Hard(ref c) | DeriveJunction::Soft(ref c) => c,
		}
	}

	pub fn is_soft(&self) -> bool {
		matches!(*self, DeriveJunction::Soft(_))
	}

	pub fn is_hard(&self) -> bool {
		matches!(*self, DeriveJunction::Hard(_))
	}
}

#[cfg(feature = "full_crypto")]
impl<T: AsRef<str>> From<T> for DeriveJunction {
	fn from(j: T) -> DeriveJunction {
		let j = j.as_ref();
		let (code, mut hard) =
			if let Some(stripped) = j.strip_suffix("'") { (stripped, true) } else { (j, false) };

		let res = if let Ok(n) = str::parse::<u32>(code) {
			if n < 0x80000000u32 {
				DeriveJunction::soft(n.to_be())
			} else {
				hard = true;
				DeriveJunction::soft((n & !(0x80000000u32)).to_be())
			}
		} else {
			DeriveJunction::soft(code)
		};

		if hard {
			res.harden()
		} else {
			res
		}
	}
}

#[cfg(feature = "full_crypto")]
impl Into<sp_core::DeriveJunction> for DeriveJunction {
	fn into(self) -> sp_core::DeriveJunction {
		let mut x = [0u8; sp_core::crypto::JUNCTION_ID_LEN];
		x.copy_from_slice(&self.inner()[0..JUNCTION_ID_LEN]);
		match self {
			DeriveJunction::Soft(_) => sp_core::DeriveJunction::Soft(x),
			DeriveJunction::Hard(_) => sp_core::DeriveJunction::Hard(x),
		}
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
		mac.update(i.inner());

		let result = mac.finalize().into_bytes();

		let mut secret = [0u8; 32];
		let mut chain_code = [0u8; 32];

		secret.copy_from_slice(&result[0..32]);
		chain_code.copy_from_slice(&result[32..]);

		secret = Self::scalar_add(&secret, self.secret());

		Ok(ExtendedPrivateKey::new(secret, chain_code))
	}
}
