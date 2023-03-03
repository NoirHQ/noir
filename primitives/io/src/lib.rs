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

/// Interfaces for working with crypto related types from within the runtime.
#[runtime_interface]
pub trait Crypto {
	/// Verify P-256 signature.
	fn p256_verify(sig: &p256::Signature, msg: &[u8], pubkey: &p256::Public) -> bool {
		p256::Pair::verify_prehashed(sig, &sp_io::hashing::sha2_256(msg), pubkey)
	}

	/// Verify WebAuthn ES256 signature.
	fn webauthn_verify(sig: &webauthn::Signature, msg: &[u8], pubkey: &webauthn::Public) -> bool {
		sig.verify(msg, pubkey)
	}
}
