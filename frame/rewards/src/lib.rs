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

//! Rewards pallet for block reward distribution.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod benchmarking;
mod mock;
mod tests;
pub mod weights;
pub use weights::WeightInfo;

pub use pallet::*;

use alloc::collections::BTreeMap;
use frame_support::traits::{Currency, LockIdentifier, LockableCurrency, WithdrawReasons};
use np_arithmetic::traits::{AtLeast32BitUnsigned, CheckedAdd, SaturatingMulDiv, Zero};
use np_rewards::{split_reward, InherentError, InherentType, INHERENT_IDENTIFIER};
use parity_scale_codec::FullCodec;
use sp_inherents::{InherentData, InherentIdentifier};

const LOCK_IDENTIFIER: LockIdentifier = *b"rewards_";

pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Coin emission schedule.
		type EmissionSchedule: Get<BalanceOf<Self>>;

		/// Currency type of this pallet.
		type Currency: LockableCurrency<Self::AccountId>;

		/// Minimum payout amount.
		///
		/// This must be greater or equal than existential deposit.
		type MinPayout: Get<BalanceOf<Self>>;

		/// Required period that newly minted coins become spendable.
		#[pallet::constant]
		type MaturationTime: Get<BlockNumberFor<Self>>;

		/// Maximum number of reward splits.
		#[pallet::constant]
		type MaxRewardSplits: Get<u32>;

		/// Miner's contribution to generate the proof of the block.
		type Share: FullCodec + Copy + AtLeast32BitUnsigned;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Invalid reward amount.
		InvalidReward,
		/// Coinbase contains too many reward splits.
		TooManyRewardSplits,
	}

	#[pallet::storage]
	pub type Processed<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn rewards)]
	pub type Rewards<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewardSplits>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn reward_locks)]
	pub type RewardLocks<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::coinbase(rewards.len() as u32))]
		pub fn coinbase(
			origin: OriginFor<T>,
			rewards: Vec<(T::AccountId, BalanceOf<T>)>,
		) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(!Processed::<T>::exists(), "multiple coinbase not allowed");
			ensure!(
				rewards.len() <= T::MaxRewardSplits::get() as usize,
				Error::<T>::TooManyRewardSplits
			);

			let reward = T::EmissionSchedule::get();
			let mut reward_given = BalanceOf::<T>::zero();
			for (dest, value) in &rewards {
				drop(T::Currency::deposit_creating(dest, *value));
				reward_given += *value;

				RewardLocks::<T>::mutate(dest, |lock| {
					let new_lock = match lock.take() {
						Some(lock) => lock + *value,
						None => *value,
					};
					T::Currency::set_lock(
						LOCK_IDENTIFIER,
						dest,
						new_lock,
						WithdrawReasons::except(WithdrawReasons::TRANSACTION_PAYMENT),
					);
					*lock = Some(new_lock);
				});
			}
			ensure!(reward_given == reward, Error::<T>::InvalidReward);

			Rewards::<T>::insert(
				frame_system::Pallet::<T>::block_number(),
				BoundedVec::<_, T::MaxRewardSplits>::try_from(rewards).unwrap(),
			);

			Processed::<T>::put(true);
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(number: BlockNumberFor<T>) -> Weight {
			let reward_splits = if number > T::MaturationTime::get() {
				{
					let unlocked_number = number - T::MaturationTime::get();

					let rewards = Rewards::<T>::take(unlocked_number);
					let reward_splits = rewards.len() as u64;

					for (dest, value) in rewards {
						RewardLocks::<T>::mutate(&dest, |lock| {
							let locked = lock.unwrap();
							if locked > value {
								T::Currency::set_lock(
									LOCK_IDENTIFIER,
									&dest,
									locked - value,
									WithdrawReasons::except(WithdrawReasons::TRANSACTION_PAYMENT),
								);
								*lock = Some(locked - value);
							} else {
								T::Currency::remove_lock(LOCK_IDENTIFIER, &dest);
								*lock = None;
							}
						});
					}
					reward_splits
				}
			} else {
				0
			};

			T::WeightInfo::on_finalize(reward_splits as u32)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			assert!(Processed::<T>::take(), "coinbase must be processed");
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T>
	where
		BalanceOf<T>: SaturatingMulDiv<T::Share>,
	{
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let mut shares = data
				.get_data::<InherentType<T::AccountId, T::Share>>(&INHERENT_IDENTIFIER)
				.expect("Rewards inherent data not correctly encoded")
				.expect("Rewards inherent data must be provided");

			let reward = T::EmissionSchedule::get();

			// Prune zero shares and ensure the number of shares is within the limit.
			shares.retain(|_, share| !share.is_zero());
			if shares.is_empty() || shares.len() > T::MaxRewardSplits::get() as usize {
				return None;
			}

			let mut rewards = split_reward(reward, shares.clone())?;

			// Prune rewards that are below the minimum payout.
			rewards.retain(|(_, value)| *value >= T::MinPayout::get());

			let rewards = if rewards.len() != shares.len() {
				// Redistribute the reward to the remaining accounts.
				shares.retain(|acc, _| rewards.iter().any(|(a, _)| a == acc));
				split_reward(reward, shares)?
			} else {
				rewards
			};

			if rewards
				.iter()
				.try_fold(BalanceOf::<T>::zero(), |sum, (_, value)| sum.checked_add(value))? !=
				reward
			{
				return None
			}

			Some(Call::coinbase { rewards })
		}

		fn check_inherent(call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
			if let Call::coinbase { rewards } = call {
				let expected_reward = T::EmissionSchedule::get();
				let reward = rewards
					.iter()
					.try_fold(BalanceOf::<T>::zero(), |sum, (_, value)| sum.checked_add(value))
					.ok_or(InherentError::InvalidReward)?;
				if reward != expected_reward {
					return Err(InherentError::InvalidReward);
				}
				if BTreeMap::from_iter(rewards.iter().cloned()).len() != rewards.len() {
					return Err(InherentError::DuplicateBeneficiary);
				}
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::coinbase { .. })
		}
	}

	impl<T: Config> Pallet<T> {
		/// Pushes the coinbase rewards. Only use for tests.
		#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
		pub fn insert_coinbase(
			number: BlockNumberFor<T>,
			rewards: Vec<(T::AccountId, BalanceOf<T>)>,
		) {
			for (dest, value) in &rewards {
				drop(T::Currency::deposit_creating(dest, *value));
				RewardLocks::<T>::mutate(dest, |lock| {
					let new_lock = match lock.take() {
						Some(lock) => lock + *value,
						None => *value,
					};
					T::Currency::set_lock(
						LOCK_IDENTIFIER,
						dest,
						new_lock,
						WithdrawReasons::except(WithdrawReasons::TRANSACTION_PAYMENT),
					);
					*lock = Some(new_lock);
				});
			}
			Rewards::<T>::insert(
				number,
				BoundedVec::<_, T::MaxRewardSplits>::try_from(rewards).unwrap(),
			);
			Processed::<T>::put(true);
		}
	}
}
