// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
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

//! WTEMA difficulty adjustment algorithm.
//!
//! <https://github.com/zawy12/difficulty-algorithms/issues/76>
//!
//! ```
//! target = prior_target * (1 + t/T/N - 1/N)
//! where
//!   N = smoothing constant aka filter
//!   t = prior block solvetime
//!   T = desired average block time
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use core::{fmt::Debug, marker::PhantomData};
use frame_support::{
	sp_runtime,
	traits::{Get, OnTimestampSet},
};
use parity_scale_codec::FullCodec;
use sp_core::U256;
use sp_runtime::traits::{One, SaturatedConversion, UniqueSaturatedFrom, UniqueSaturatedInto};

/// Helper type to calculate the minimum difficulty.
pub struct MinDifficulty<T>(PhantomData<T>);
impl<T> Get<T::Difficulty> for MinDifficulty<T>
where
	T: Config,
	T::Moment: UniqueSaturatedInto<T::Difficulty>,
{
	fn get() -> T::Difficulty {
		let filter = T::Moment::from(T::Filter::get());
		let target_block_time = T::TargetBlockTime::get();
		let minimum_period = T::MinimumPeriod::get();

		((filter * target_block_time - T::Moment::one()) / (target_block_time - minimum_period))
			.saturated_into()
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		/// Difficulty for cryptographic puzzles in PoW consensus.
		type Difficulty: FullCodec
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug
			+ MaxEncodedLen
			+ TypeInfo
			+ UniqueSaturatedFrom<U256>
			+ Into<U256>
			+ PartialOrd;

		/// Desired block time in milliseconds.
		#[pallet::constant]
		type TargetBlockTime: Get<Self::Moment>;

		/// Smoothing constant for difficulty adjustment.
		#[pallet::constant]
		type Filter: Get<u32>;

		/// Minimum difficulty to be adjusted according to block time changes.
		///
		/// If the difficulty drops below the minimum difficulty, it stops adjusting because of
		/// rounding errors.
		#[pallet::constant]
		type MinDifficulty: Get<Self::Difficulty>;
	}

	/// Target difficulty for the next block.
	#[pallet::storage]
	#[pallet::getter(fn difficulty)]
	pub type Difficulty<T: Config> = StorageValue<_, T::Difficulty, ValueQuery>;

	/// Timestamp of the last block.
	#[pallet::storage]
	#[pallet::getter(fn last_timestamp)]
	pub type LastTimestamp<T: Config> = StorageValue<_, T::Moment, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_difficulty: T::Difficulty,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { initial_difficulty: T::Difficulty::saturated_from(U256::from(10000)) }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			assert!(self.initial_difficulty.into() != U256::from(0));
			Difficulty::<T>::put(self.initial_difficulty);
		}
	}
}

impl<T: Config> OnTimestampSet<T::Moment> for Pallet<T>
where
	T::Moment: Into<U256>,
{
	fn on_timestamp_set(now: T::Moment) {
		let block_time = match frame_system::Pallet::<T>::block_number() {
			n if n <= One::one() => T::TargetBlockTime::get(),
			_ => now - LastTimestamp::<T>::get(),
		};
		let desired_block_time = T::TargetBlockTime::get().into();
		let prior_target = U256::max_value() / Difficulty::<T>::get().into();
		let filter = T::Filter::get();

		let target = (prior_target / (desired_block_time * filter))
			.saturating_mul(desired_block_time * filter + block_time - desired_block_time);
		let mut difficulty = T::Difficulty::saturated_from(U256::max_value() / target);

		if difficulty < T::MinDifficulty::get() {
			difficulty = T::MinDifficulty::get();
		}

		Difficulty::<T>::put(difficulty);
		LastTimestamp::<T>::put(now);
	}
}
