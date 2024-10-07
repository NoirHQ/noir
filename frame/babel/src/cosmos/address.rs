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

use crate::{extensions::unify_account, VarAddress};
use alloc::{string::String, vec::Vec};
use bech32::{Bech32, Hrp};
use core::marker::PhantomData;
use np_babel::cosmos::{traits::ChainInfo, Address as CosmosAddress};
use pallet_cosmos_types::address::acc_address_from_bech32;
use pallet_cosmwasm::types::AccountIdOf;
use pallet_multimap::traits::UniqueMultimap;
use sp_core::{H160, H256};
use sp_runtime::traits::{AccountIdConversion, Convert};

pub struct AddressMapping<T>(PhantomData<T>);
impl<T> pallet_cosmos::AddressMapping<T::AccountId> for AddressMapping<T>
where
	T: unify_account::Config,
{
	fn into_account_id(who: H160) -> T::AccountId {
		let address = CosmosAddress::from(who);
		T::AddressMap::find_key(VarAddress::Cosmos(address.clone()))
			.unwrap_or_else(|| address.into_account_truncating())
	}
}

pub struct AccountToAddr<T>(PhantomData<T>);
impl<T> Convert<AccountIdOf<T>, String> for AccountToAddr<T>
where
	T: pallet_cosmwasm::Config + unify_account::Config,
{
	fn convert(account: AccountIdOf<T>) -> String {
		let addresses = T::AddressMap::get(account.clone());
		let address: Option<&CosmosAddress> = addresses.iter().find_map(|address| match address {
			VarAddress::Cosmos(address) => Some(address),
			_ => None,
		});
		let address_raw = match address {
			Some(address) => address.as_ref(),
			None => account.as_ref(),
		};
		let hrp = Hrp::parse(T::ChainInfo::bech32_prefix()).unwrap();
		bech32::encode::<Bech32>(hrp, address_raw).unwrap()
	}
}

impl<T> Convert<String, Result<AccountIdOf<T>, ()>> for AccountToAddr<T>
where
	T: pallet_cosmwasm::Config + unify_account::Config,
{
	fn convert(address: String) -> Result<AccountIdOf<T>, ()> {
		let (_hrp, address_raw) = acc_address_from_bech32(&address).map_err(|_| ())?;
		Self::convert(address_raw)
	}
}

impl<T> Convert<Vec<u8>, Result<AccountIdOf<T>, ()>> for AccountToAddr<T>
where
	T: pallet_cosmwasm::Config + unify_account::Config,
{
	fn convert(address: Vec<u8>) -> Result<AccountIdOf<T>, ()> {
		match address.len() {
			20 => {
				let address = CosmosAddress::from(H160::from_slice(&address));
				T::AddressMap::find_key(VarAddress::Cosmos(address)).ok_or(())
			},
			32 => Ok(H256::from_slice(&address).into()),
			_ => Err(()),
		}
	}
}
