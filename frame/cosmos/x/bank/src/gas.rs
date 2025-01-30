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

use core::marker::PhantomData;
use frame_support::weights::Weight;
use pallet_cosmos_types::gas::Gas;
use sp_core::Get;
use sp_runtime::traits::Convert;

pub struct GasInfo<T>(PhantomData<T>);
impl<T: pallet_cosmos::Config> GasInfo<T> {
	pub fn msg_send_native() -> Gas {
		let weight = Weight::from_parts(61_290_000, 3593)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64));
		T::WeightToGas::convert(weight)
	}

	pub fn msg_send_asset() -> Gas {
		let weight = Weight::from_parts(40_059_000, 6208)
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64));
		T::WeightToGas::convert(weight)
	}
}
