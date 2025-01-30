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

use crate as pallet_multimap;
use frame_support::{derive_impl, sp_runtime, traits::ConstU32};
use sp_io::TestExternalities;
use sp_runtime::BuildStorage;

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(RuntimeCall, RuntimeEvent, RuntimeError, RuntimeOrigin, RuntimeTask)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Multimap = pallet_multimap;
}

type Block = frame_system::mocking::MockBlock<Test>;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

#[derive_impl(pallet_multimap::config_preludes::TestDefaultConfig)]
impl pallet_multimap::Config for Test {
	type Key = <Test as frame_system::Config>::AccountId;
	type Value = u32;
	type CapacityPerKey = ConstU32<2>;
}

pub fn new_test_ext() -> TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	TestExternalities::new(t)
}
