// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
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

//! Common implementation across Noir runtimes.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod units;

use crate::units::MiB;
use frame_support::{
	parameter_types,
	sp_runtime::{traits::Bounded, FixedPointNumber, Perbill, Perquintill},
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use frame_system::limits;
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use static_assertions::const_assert;

/// Assumes that 1% of the block weight is used for initialization.
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(1);
/// 75% of the block weight is used for `Normal` extrinsics.
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// 2 seconds for block computation with 6-second average block time.
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

parameter_types! {
	/// 4096 block hashes are kept in storage.
	pub const BlockHashCount: u32 = 4096;
	/// The portion of `NORMAL_DISPATCH_RATIO` for adjusting fees.
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	/// Adjustment variable that how much the fee should be adjusted.
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(75, 1_000_000);
	/// Minimum amount of the multiplier. This value should be high enough to recover from the
	/// minimum.
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 10u128);
	/// Maximum amount of the multiplier.
	pub MaximumMultiplier: Multiplier = Bounded::max_value();
	/// 5 MiB block size limit.
	pub BlockLength: limits::BlockLength =
		limits::BlockLength::max_with_normal_ratio(5 * MiB, NORMAL_DISPATCH_RATIO);
}

/// Parameterized slow adjusting fee updated.
pub type SlowAdjustingFeeUpdate<R> = TargetedFeeAdjustment<
	R,
	TargetBlockFullness,
	AdjustmentVariable,
	MinimumMultiplier,
	MaximumMultiplier,
>;
