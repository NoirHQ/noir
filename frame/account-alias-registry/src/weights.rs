// This file is part of Noir.

// Copyright (C) 2023 Haderech Pte. Ltd.
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

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use core::marker::PhantomData;
use frame_support::{traits::Get, weights::Weight};

/// Weight functions needed for pallet_account_alias_registy.
pub trait WeightInfo {
	fn create_account_name() -> Weight;
	fn update_account_name() -> Weight;
	fn connect_aliases() -> Weight;
	fn force_set_account_name() -> Weight;
}

/// Weights for pallet_account_alias_registy using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: AccountNames (r:1 w:1), AccountNameIndex (r:1 w:1)
	fn create_account_name() -> Weight {
    // Base fee
		Weight::from_ref_time(50_000_000)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}	
  // Storage: AccountNames (r:1 w:2), AccountNameIndex (r:1 w:1)
	fn update_account_name() -> Weight {
    // Base fee
		Weight::from_ref_time(100_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: AccountNames (r:1 w:1), AccountNameIndex (r:1 w:1)
	fn connect_aliases() -> Weight {
		// Base fee
		Weight::from_ref_time(50_000_000)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}	
	// Storage: AccountNames (r:1 w:2), AccountNameIndex (r:1 w:1)
	fn force_set_account_name() -> Weight {
		// Base fee
		Weight::from_ref_time(50_000_000)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}	
}
