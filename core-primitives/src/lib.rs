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

//! Noir core primitives.

#![cfg_attr(not(feature = "std"), no_std)]

use np_runtime::{
	traits::{IdentifyAccount, VerifyMut},
	MultiSignature,
};
use sp_core::H256;

pub use sp_runtime::traits::BlakeTwo256;

/// Account identifier.
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

/// Sequential index of an account (not used).
pub type AccountIndex = ();

/// Transaction counter (nonce).
pub type AccountNonce = u32;

/// Account public key.
pub type AccountPublic = <Signature as VerifyMut>::Signer;

/// Native currency balance.
pub type Balance = u128;

/// Block index.
pub type BlockNumber = u32;

/// 256-bit hash.
pub type Hash = H256;

/// 64-bit timestamp.
pub type Moment = u64;

/// Digital signature.
pub type Signature = MultiSignature;

/// Opaque types.
pub mod opaque {
	use super::*;
	use sp_runtime::{generic, OpaqueExtrinsic};

	/// Opaque block.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;

	/// Opaque block identifier.
	pub type BlockId = generic::BlockId<Block>;

	/// Opaque block header.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

	/// Opaque extrinsic.
	pub type UncheckedExtrinsic = OpaqueExtrinsic;
}
