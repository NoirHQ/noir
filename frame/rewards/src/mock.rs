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

#![cfg(test)]

use crate as pallet_rewards;

use super::*;
use frame_support::{
	derive_impl,
	sp_runtime::BuildStorage,
	traits::{ConstU32, ConstU64},
};
use sp_io::TestExternalities;

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeHoldReason,
		RuntimeFreezeReason,
		RuntimeOrigin,
		RuntimeTask
	)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Rewards = pallet_rewards;

	#[runtime::pallet_index(2)]
	pub type Balances = pallet_balances;
}

type Block = frame_system::mocking::MockBlock<Test>;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

impl Config for Test {
	type EmissionSchedule = ConstU64<100>;
	type Currency = Balances;
	type MinPayout = ConstU64<1>;
	type MaturationTime = ConstU64<1>;
	type MaxRewardSplits = ConstU32<100>;
	type Share = u128;
	type WeightInfo = ();
}

pub fn new_test_ext() -> TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	TestExternalities::new(t)
}
