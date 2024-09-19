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

use crate::{extensions::unify_account, Address};
use core::marker::PhantomData;
#[cfg(feature = "cosmos")]
pub use np_cosmos::Address as CosmosAddress;
use pallet_multimap::traits::UniqueMultimap;
use sp_core::H160;
use sp_runtime::traits::AccountIdConversion;

pub struct AddressMapping<T>(PhantomData<T>);

impl<T> pallet_evm::AddressMapping<T::AccountId> for AddressMapping<T>
where
	T: unify_account::Config,
{
	fn into_account_id(who: H160) -> T::AccountId {
		let address = CosmosAddress::from(who);
		T::AddressMap::find_key(Address::Cosmos(address.clone()))
			.unwrap_or_else(|| address.into_account_truncating())
	}
}
