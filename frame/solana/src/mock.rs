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

use crate as pallet_solana;

use frame_support::{
	derive_impl,
	sp_runtime::{traits::ConstU128, BuildStorage},
};

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(1)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(10)]
	pub type Solana = pallet_solana;
}

type Balance = u128;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = frame_system::mocking::MockBlock<Self>;
	type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
}

#[derive_impl(pallet_solana::config_preludes::TestDefaultConfig)]
impl pallet_solana::Config for Test {
	type Balance = <Self as pallet_balances::Config>::Balance;
	type Currency = Balances;
	type DecimalMultiplier = ConstU128<1_000_000_000>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	t.into()
}
