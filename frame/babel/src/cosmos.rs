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
use alloc::{
	boxed::Box,
	string::{String, ToString},
	vec::Vec,
};
use bech32::{Bech32, Hrp};
use core::marker::PhantomData;
use cosmos_sdk_proto::{
	cosmos::bank::v1beta1::MsgSend,
	cosmwasm::wasm::v1::{
		MsgExecuteContract, MsgInstantiateContract2, MsgMigrateContract, MsgStoreCode,
		MsgUpdateAdmin,
	},
	Any,
};
use np_cosmos::traits::ChainInfo;
pub use np_cosmos::Address as CosmosAddress;
use pallet_cosmos::types::DenomOf;
use pallet_cosmos_types::{address::acc_address_from_bech32, any_match, context, msgservice};
use pallet_cosmos_x_bank::MsgSendHandler;
use pallet_cosmos_x_wasm::msgs::{
	MsgExecuteContractHandler, MsgInstantiateContract2Handler, MsgMigrateContractHandler,
	MsgStoreCodeHandler, MsgUpdateAdminHandler,
};
use pallet_multimap::traits::UniqueMultimap;
use sp_core::{Get, H160, H256};
use sp_runtime::traits::{AccountIdConversion, Convert};

pub struct AddressMapping<T>(PhantomData<T>);
impl<T> pallet_cosmos::AddressMapping<T::AccountId> for AddressMapping<T>
where
	T: unify_account::Config,
{
	fn into_account_id(who: H160) -> T::AccountId {
		let address = CosmosAddress::from(who);
		T::AddressMap::find_key(Address::Cosmos(address.clone()))
			.unwrap_or_else(|| address.into_account_truncating())
	}
}

pub struct AssetToDenom<T>(PhantomData<T>);
impl<T> Convert<String, Result<T::AssetId, ()>> for AssetToDenom<T>
where
	T: pallet_cosmos::Config,
{
	fn convert(denom: String) -> Result<T::AssetId, ()> {
		if denom == T::NativeDenom::get() {
			Ok(T::NativeAssetId::get())
		} else {
			let denom: DenomOf<T> = denom.as_bytes().to_vec().try_into().map_err(|_| ())?;
			pallet_cosmos::DenomAssetRouter::<T>::get(denom).ok_or(())
		}
	}
}
impl<T> Convert<T::AssetId, String> for AssetToDenom<T>
where
	T: pallet_cosmos::Config,
{
	fn convert(asset_id: T::AssetId) -> String {
		if asset_id == T::NativeAssetId::get() {
			T::NativeDenom::get().to_string()
		} else {
			// TODO: Handle option
			let denom = pallet_cosmos::AssetDenomRouter::<T>::get(asset_id).unwrap().to_vec();
			String::from_utf8(denom).unwrap()
		}
	}
}

pub struct MsgServiceRouter<T>(PhantomData<T>);
impl<T, Context> msgservice::traits::MsgServiceRouter<Context> for MsgServiceRouter<T>
where
	T: frame_system::Config + pallet_cosmos::Config + pallet_cosmwasm::Config,
	Context: context::traits::Context,
{
	fn route(msg: &Any) -> Option<Box<dyn msgservice::traits::MsgHandler<Context>>> {
		any_match!(
			msg, {
				MsgSend => Some(Box::<MsgSendHandler<T>>::default()),
				MsgStoreCode => Some(Box::<MsgStoreCodeHandler<T>>::default()),
				MsgInstantiateContract2 => Some(Box::<MsgInstantiateContract2Handler<T>>::default()),
				MsgExecuteContract => Some(Box::<MsgExecuteContractHandler<T>>::default()),
				MsgMigrateContract => Some(Box::<MsgMigrateContractHandler<T>>::default()),
				MsgUpdateAdmin => Some(Box::<MsgUpdateAdminHandler<T>>::default()),
			},
			None
		)
	}
}

pub struct AccountToAddr<T>(PhantomData<T>);
impl<T> Convert<T::AccountIdExtended, String> for AccountToAddr<T>
where
	T: pallet_cosmwasm::Config + unify_account::Config,
{
	fn convert(account: T::AccountIdExtended) -> String {
		let addresses = T::AddressMap::get(account.clone());
		let address: Option<&CosmosAddress> = addresses.iter().find_map(|address| match address {
			Address::Cosmos(address) => Some(address),
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
impl<T> Convert<String, Result<T::AccountIdExtended, ()>> for AccountToAddr<T>
where
	T: pallet_cosmwasm::Config + unify_account::Config,
{
	fn convert(address: String) -> Result<T::AccountIdExtended, ()> {
		let (_hrp, address_raw) = acc_address_from_bech32(&address).map_err(|_| ())?;
		Self::convert(address_raw)
	}
}
impl<T> Convert<Vec<u8>, Result<T::AccountIdExtended, ()>> for AccountToAddr<T>
where
	T: pallet_cosmwasm::Config + unify_account::Config,
	T::AccountIdExtended: From<H256>,
{
	fn convert(address: Vec<u8>) -> Result<T::AccountIdExtended, ()> {
		match address.len() {
			20 => {
				let address = CosmosAddress::from(H160::from_slice(&address));
				T::AddressMap::find_key(Address::Cosmos(address)).ok_or(())
			},
			32 => Ok(H256::from_slice(&address).into()),
			_ => return Err(()),
		}
	}
}
