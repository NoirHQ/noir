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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
extern crate alloc;

#[cfg(feature = "cosmos")]
pub mod cosmos;
#[cfg(feature = "ethereum")]
pub mod ethereum;
pub mod extensions;

pub use extensions::unify_account::UnifyAccount;
pub use np_babel::VarAddress;

#[cfg(feature = "pallet")]
pub use pallet::*;

#[cfg(feature = "pallet")]
#[frame_support::pallet]
pub mod pallet {
	use super::VarAddress;
	use alloc::vec::Vec;
	use cosmos_sdk_proto::{cosmos::tx::v1beta1::Tx, traits::Message};
	use frame_support::{
		pallet_prelude::*,
		traits::{fungible::Mutate, tokens::Preservation::Preserve},
	};
	use frame_system::{ensure_root, pallet_prelude::*};
	use pallet_cosmos::{
		types::{AssetIdOf, DenomOf},
		AddressMapping as _,
	};
	use pallet_cosmos_types::address::acc_address_from_bech32;
	use pallet_cosmos_x_auth_signing::sign_verifiable_tx::traits::SigVerifiableTx;
	use pallet_evm::AddressMapping as _;
	use pallet_multimap::traits::{UniqueMap, UniqueMultimap};
	use sp_core::ecdsa;
	use sp_runtime::{
		traits::{StaticLookup, UniqueSaturatedInto},
		AccountId32,
	};

	type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ pallet_assets::Config
		+ pallet_balances::Config
		+ pallet_cosmos::Config<AssetId = <Self as pallet_assets::Config>::AssetId>
		+ pallet_ethereum::Config
		+ pallet_evm::Config
	{
		type AddressMap: UniqueMultimap<Self::AccountId, VarAddress>;
		type AssetMap: UniqueMap<AssetIdOf<Self>, DenomOf<Self>>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {
		InvalidOrigin,
		InvalidTransaction,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		OriginFor<T>: Into<Result<pallet_ethereum::RawOrigin, OriginFor<T>>>
			+ Into<Result<pallet_cosmos::RawOrigin, OriginFor<T>>>,
		T::AccountId: TryInto<ecdsa::Public> + From<AccountId32>,
		T::RuntimeOrigin: From<pallet_ethereum::RawOrigin> + From<pallet_cosmos::RawOrigin>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight({
			use ethereum::EnvelopedDecodable;
			use pallet_ethereum::{Transaction, TransactionData};
			use pallet_evm::GasWeightMapping;
			let without_base_extrinsic_weight = true;
			match <Transaction as EnvelopedDecodable>::decode(transaction) {
				Ok(transaction) => {
					<T as pallet_evm::Config>::GasWeightMapping::gas_to_weight({
						let transaction_data = TransactionData::from(&transaction);
						transaction_data.gas_limit.unique_saturated_into()
						}, without_base_extrinsic_weight)
				},
				Err(_) => Weight::MAX,
			}
		})]
		pub fn ethereum_transact(
			origin: OriginFor<T>,
			transaction: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let address = T::AddressMap::get(who)
				.iter()
				.find_map(|address| match address {
					VarAddress::Ethereum(address) => Some(address.clone()),
					_ => None,
				})
				.ok_or(Error::<T>::InvalidOrigin)?;

			let origin = T::RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(
				address.into(),
			));
			let transaction = ethereum::EnvelopedDecodable::decode(&transaction)
				.map_err(|_| Error::<T>::InvalidTransaction)?;

			pallet_ethereum::Pallet::<T>::transact(origin, transaction)
		}

		#[pallet::call_index(1)]
		#[pallet::weight({
			use cosmos_sdk_proto::traits::Message;
			use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx;
			use pallet_cosmos::weights::WeightInfo;
			use sp_runtime::traits::Convert;

			Tx::decode(&mut &tx_bytes[..])
				.ok()
				.and_then(|tx| tx.auth_info)
				.and_then(|auth_info| auth_info.fee)
				.map_or(<T as pallet_cosmos::Config>::WeightInfo::base_weight(), |fee| {
					<T as pallet_cosmos::Config>::WeightToGas::convert(fee.gas_limit)
				})
		})]
		pub fn cosmos_transact(
			origin: OriginFor<T>,
			tx_bytes: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let address = T::AddressMap::get(who)
				.iter()
				.find_map(|address| match address {
					VarAddress::Cosmos(address) => Some(address.clone()),
					_ => None,
				})
				.ok_or(Error::<T>::InvalidOrigin)?;

			let tx = Tx::decode(&mut &*tx_bytes).map_err(|_| Error::<T>::InvalidTransaction)?;
			let signers =
				T::SigVerifiableTx::get_signers(&tx).map_err(|_| Error::<T>::InvalidTransaction)?;
			ensure!(signers.len() == 1, Error::<T>::InvalidTransaction);

			let signer = signers.first().ok_or(Error::<T>::InvalidTransaction)?;
			let (_hrp, address_raw) =
				acc_address_from_bech32(signer).map_err(|_| Error::<T>::InvalidTransaction)?;
			ensure!(
				address_raw.len() == 20 && address.to_vec() == address_raw,
				Error::<T>::InvalidTransaction
			);

			let origin =
				T::RuntimeOrigin::from(pallet_cosmos::RawOrigin::CosmosTransaction(address.into()));

			pallet_cosmos::Pallet::<T>::transact(origin, tx_bytes)
		}

		#[pallet::call_index(2)]
		#[pallet::weight({
			use pallet_assets::weights::WeightInfo;

			<T as pallet_assets::Config>::WeightInfo::force_create()
				.saturating_add(<T as pallet_assets::Config>::WeightInfo::force_set_metadata(name.len() as u32, symbol.len() as u32))
		})]
		pub fn force_create_asset(
			origin: OriginFor<T>,
			id: T::AssetIdParameter,
			name: Vec<u8>,
			symbol: Vec<u8>,
			denom: Vec<u8>,
			decimals: u8,
			is_frozen: bool,
			is_sufficient: bool,
			owner: AccountIdLookupOf<T>,
			#[pallet::compact] min_balance: <T as pallet_assets::Config>::Balance,
		) -> DispatchResult {
			ensure_root(origin.clone())?;

			pallet_assets::Pallet::<T>::force_create(
				origin.clone(),
				id.clone(),
				owner,
				is_sufficient,
				min_balance,
			)?;
			pallet_assets::Pallet::<T>::force_set_metadata(
				origin,
				id.clone(),
				symbol,
				name,
				decimals,
				is_frozen,
			)?;
			let id: <T as pallet_cosmos::Config>::AssetId = id.into();
			let denom: DenomOf<T> =
				denom.try_into().map_err(|_| DispatchError::Other("Too long denom"))?;
			T::AssetMap::try_insert(id, denom)
				.map_err(|_| DispatchError::Other("Failed to insert into asset map"))?;

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight({
			use pallet_balances::weights::WeightInfo;

			<T as pallet_balances::Config>::WeightInfo::transfer_keep_alive()
		})]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: VarAddress,
			#[pallet::compact] value: <T as pallet_balances::Config>::Balance,
		) -> DispatchResult {
			let source = ensure_signed(origin.clone())?;

			let dest: T::AccountId = match dest {
				VarAddress::Cosmos(address) =>
					<T as pallet_cosmos::Config>::AddressMapping::into_account_id(address.into()),
				VarAddress::Ethereum(address) =>
					<T as pallet_evm::Config>::AddressMapping::into_account_id(address.into()),
				VarAddress::Polkadot(address) => address.into(),
			};

			pallet_balances::Pallet::<T>::transfer(&source, &dest, value, Preserve).map(|_| ())
		}
	}
}
