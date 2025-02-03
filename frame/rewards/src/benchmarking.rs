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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::{
	sp_runtime::traits::{Get, One},
	traits::Hooks,
};
use frame_system::RawOrigin;

const SEED: u32 = 0;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn coinbase(n: Linear<1, { T::MaxRewardSplits::get() }>) -> Result<(), BenchmarkError> {
		type System<T> = frame_system::Pallet<T>;

		let mut reward = T::EmissionSchedule::get();
		let mut rewards: Vec<(T::AccountId, BalanceOf<T>)> = Vec::new();
		let payout = T::MinPayout::get();
		for index in 1..n {
			reward -= payout;
			rewards.push((account("miner", index, SEED), payout));
		}
		let payout = reward;
		rewards.push((account("miner", n, SEED), reward));

		#[extrinsic_call]
		_(RawOrigin::None, rewards.clone());

		assert_eq!(Processed::<T>::get(), true);
		assert_eq!(Rewards::<T>::get(System::<T>::block_number()), rewards);
		assert_eq!(RewardLocks::<T>::get(account::<T::AccountId>("miner", n, SEED)), Some(payout));

		Ok(())
	}

	#[benchmark]
	fn on_finalize(n: Linear<1, { T::MaxRewardSplits::get() }>) {
		type Rewards<T> = Pallet<T>;
		type System<T> = frame_system::Pallet<T>;

		let number = System::<T>::block_number() + One::one();
		let mut reward = T::EmissionSchedule::get();
		let mut rewards: Vec<(T::AccountId, BalanceOf<T>)> = Vec::new();
		let payout = T::MinPayout::get();
		for index in 1..n {
			reward -= payout;
			rewards.push((account("miner", index, SEED), payout));
		}
		rewards.push((account("miner", n, SEED), reward));
		Rewards::<T>::insert_coinbase(number, rewards);

		#[block]
		{
			Rewards::<T>::on_finalize(number);
			Rewards::<T>::on_initialize(number + T::MaturationTime::get());
		}
	}

	impl_benchmark_test_suite!(Rewards, crate::mock::new_test_ext(), crate::mock::Text,);
}
