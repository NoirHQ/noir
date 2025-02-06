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
