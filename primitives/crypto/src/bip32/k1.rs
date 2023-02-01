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

//! BIP32 for Secp256k1 curve.

#![cfg(feature = "full_crypto")]

use k256::{
	elliptic_curve::{ops::Add, sec1::ToEncodedPoint, ScalarCore},
	Secp256k1, SecretKey,
};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, PartialEq, Eq, Debug, Zeroize, ZeroizeOnDrop)]
pub struct ExtendedPrivateKey([u8; 32], [u8; 32]);

impl crate::bip32::ExtendedPrivateKey for ExtendedPrivateKey {
	fn new(secret: [u8; 32], chain_code: [u8; 32]) -> Self {
		Self(secret, chain_code)
	}

	fn secret(&self) -> &[u8] {
		&self.0[..]
	}

	fn chain_code(&self) -> &[u8] {
		&self.1[..]
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
