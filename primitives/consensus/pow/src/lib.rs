// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
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

//! Noir primitive types for PoW consensus.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::ops::AddAssign;
use parity_scale_codec::{Decode, Encode};
use sp_api::decl_runtime_apis;
use sp_arithmetic::traits::{Bounded, SaturatedConversion, Saturating, UniqueSaturatedFrom};
use sp_core::U256;
use sp_runtime::ConsensusEngineId;

/// `ConsensusEngineId` for PoW.
pub const POW_ENGINE_ID: ConsensusEngineId = *b"pow_";

/// Seal for PoW.
pub type Seal = Vec<u8>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Decode, Encode)]
pub struct BlockWeight<Weight>(Weight);

impl<Weight> AddAssign<Weight> for BlockWeight<Weight>
where
	Weight: Copy + Saturating,
{
	fn add_assign(&mut self, other: Weight) {
		let ret = self.0.saturating_add(other);
		*self = Self(ret);
	}
}

/// Checks if a hash fits the given difficulty.
pub fn check_hash<Hash, Difficulty>(hash: &Hash, difficulty: Difficulty) -> bool
where
	Hash: AsRef<[u8]>,
	Difficulty: Into<U256>,
{
	let hash = U256::from_big_endian(hash.as_ref());
	let (_, overflowed) = hash.overflowing_mul(difficulty.into());

	!overflowed
}

/// Returns a difficulty for the given hash.
pub fn difficulty<Hash, Difficulty>(hash: &Hash) -> Difficulty
where
	Hash: AsRef<[u8]>,
	Difficulty: Bounded + UniqueSaturatedFrom<U256>,
{
	let is_zero = hash.as_ref().iter().all(|&x| x == 0);

	if !is_zero {
		Difficulty::saturated_from(U256::max_value() / U256::from_big_endian(hash.as_ref()))
	} else {
		Difficulty::max_value()
	}
}

decl_runtime_apis! {
	/// API necessary for timestamp-based difficulty adjustment algorithms.
	pub trait TimestampApi<Moment: Decode> {
		/// Return the timestamp in the current block.
		fn timestamp() -> Moment;
	}

	/// API for those chains that put their difficulty adjustment algorithm directly
	/// onto runtime. Note that while putting difficulty adjustment algorithm to
	/// runtime is safe, putting the PoW algorithm on runtime is not.
	pub trait DifficultyApi<Difficulty: Decode> {
		/// Return the target difficulty of the next block.
		fn difficulty() -> Difficulty;
	}
}
