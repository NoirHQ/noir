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

//! I/O host interface for Noir runtime.

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

use np_crypto::{p256, webauthn};
use sp_runtime_interface::runtime_interface;

#[cfg(feature = "std")]
use secp256k1::PublicKey;

/// Interfaces for working with crypto related types from within the runtime.
#[runtime_interface]
pub trait Crypto {
	/// Verify P-256 signature.
	fn p256_verify(sig: &p256::Signature, msg: &[u8], pubkey: &p256::Public) -> bool {
		p256::Pair::verify_prehashed(sig, &sp_io::hashing::sha2_256(msg), pubkey)
	}

	/// Verify and recover a P-256 signature.
	fn p256_recover_compressed(sig: &[u8; 65], msg: &[u8; 32]) -> Option<[u8; 33]> {
		p256::Signature::from_raw(*sig).recover_prehashed(msg).map(|pubkey| pubkey.0)
	}

	/// Verify WebAuthn ES256 signature.
	fn webauthn_verify(sig: &webauthn::Signature, msg: &[u8], pubkey: &webauthn::Public) -> bool {
		sig.verify(msg, pubkey)
	}

	/// Verify WebAuthn ES256 signature.
	fn webauthn_recover(sig: &webauthn::Signature, msg: &[u8]) -> Option<[u8; 33]> {
		sig.recover(msg).map(|pubkey| pubkey.0)
	}

	/// Decompress secp256k1 public key.
	fn secp256k1_pubkey_serialize(pubkey: &[u8; 33]) -> Option<[u8; 64]> {
		let pubkey = PublicKey::from_slice(&pubkey[..]).ok()?;
		let mut res = [0u8; 64];
		res.copy_from_slice(&pubkey.serialize_uncompressed()[1..]);
		Some(res)
	}

	/// Hash with ripemd160.
	fn ripemd160(msg: &[u8]) -> [u8; 20] {
		hp_crypto::ripemd160(msg)
	}

	/// Verify with secp256k1.
	fn secp256k1_ecdsa_verify(sig: &[u8], msg: &[u8], pk: &[u8]) -> bool {
		hp_crypto::secp256k1_ecdsa_verify(sig, msg, pk)
	}
}
