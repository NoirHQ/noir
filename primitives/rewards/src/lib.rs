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

//! Noir primitive types for rewards distribution.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod inherents;
pub use inherents::*;

use alloc::{collections::BTreeMap, vec::Vec};
use np_arithmetic::traits::{BaseArithmetic, SaturatingMulDiv};

/// Distributes the total reward according to the weight of each account.
pub fn split_reward<Balance, AccountId, Share>(
	reward: Balance,
	shares: BTreeMap<AccountId, Share>,
) -> Option<Vec<(AccountId, Balance)>>
where
	Balance: BaseArithmetic + SaturatingMulDiv<Share> + Copy,
	Share: BaseArithmetic + Copy,
{
	let total_weight = shares
		.values()
		.try_fold(Share::zero(), |sum, weight| sum.checked_add(weight))
		.filter(|sum| !sum.is_zero())?;

	let mut rewards = Vec::new();
	let mut reward_given = Balance::zero();
	let mut cumulative_weight = Share::zero();

	for (dest, weight) in shares {
		cumulative_weight += weight;
		let next_value = reward.saturating_mul_div(cumulative_weight, total_weight);
		rewards.push((dest, next_value - reward_given));
		reward_given = next_value;
	}

	Some(rewards)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn split_reward_should_works() {
		// normal case
		{
			let shares = BTreeMap::<_, u32>::from([(1, 2), (2, 3), (3, 5)]);
			let reward = shares.values().sum();
			assert_eq!(Some(vec![(1, 2), (2, 3), (3, 5)]), split_reward(reward, shares));

			let shares = BTreeMap::<_, u8>::from([(1, 2), (2, 3), (3, 5)]);
			let reward = 10u32;
			assert_eq!(Some(vec![(1, 2), (2, 3), (3, 5)]), split_reward(reward, shares));

			let shares = BTreeMap::<_, u32>::from([(1, 2), (2, 3), (3, 5)]);
			let reward = 10u8;
			assert_eq!(Some(vec![(1, 2), (2, 3), (3, 5)]), split_reward(reward, shares));
		}

		// zero total weight
		{
			let shares = BTreeMap::<_, u32>::from([(1, 0), (2, 0), (3, 0)]);
			let reward = 10u32;
			assert_eq!(None, split_reward(reward, shares));
		}

		// total weight overflow
		{
			let shares = BTreeMap::from([(1, u32::MAX), (2, u32::MAX), (3, u32::MAX)]);
			let reward = 10u32;
			assert_eq!(None, split_reward(reward, shares));
		}

		// multiplication overflow
		{
			let shares = BTreeMap::<_, u32>::from([(1, 5), (2, 5), (3, 0)]);
			let reward = 255u8;
			assert_eq!(Some(vec![(1, 127), (2, 128), (3, 0)]), split_reward(reward, shares));
		}
	}
}
