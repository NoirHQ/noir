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

//! # Account Alias Registry Pallet

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;

use crate::weights::WeightInfo;
use codec::{Decode, Encode, MaxEncodedLen};
use np_crypto::ecdsa::EcdsaExt;
use np_runtime::AccountName;
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{LookupError, StaticLookup},
	DispatchError, MultiAddress,
};
use sp_std::prelude::*;

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

/// A generator for tag number that discriminates the same name accounts.
pub trait TagGenerator<T: Config> {
	fn tag(id: &T::AccountId, name: &str) -> Result<u16, ()>;
}

#[cfg_attr(feature = "std", derive(Hash))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum AccountAlias {
	AccountName(AccountName),
	EthereumAddress([u8; 20]),
	CosmosAddress([u8; 20]),
}

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// The module's config trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
		/// The generator for tag number that discriminates the same name accounts.
		type TagGenerator: TagGenerator<Self>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: EcdsaExt,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_account_name())]
		pub fn create_account_name(origin: OriginFor<T>, name: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(AccountNameIndex::<T>::get(&who).is_none(), Error::<T>::AlreadyExists);
			let name =
				sp_std::str::from_utf8(&name[..]).map_err(|_| Error::<T>::InvalidNameFormat)?;
			let tag =
				T::TagGenerator::tag(&who, name).map_err(|_| Error::<T>::TagGenerationFailed)?;
			let account_name =
				AccountName::new(&name, tag).map_err(|_| Error::<T>::InvalidNameFormat)?;
			AccountAliases::<T>::try_mutate(
				AccountAlias::AccountName(account_name),
				|maybe_value| -> DispatchResult {
					ensure!(maybe_value.is_none(), Error::<T>::InUse);
					*maybe_value = Some(who.clone());
					Ok(())
				},
			)?;
			AccountNameIndex::<T>::insert(&who, account_name);
			Self::deposit_event(Event::<T>::AccountNameUpdated {
				who,
				name: account_name,
				deleted: None,
			});
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_account_name())]
		pub fn update_account_name(origin: OriginFor<T>, new_name: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let account_name = AccountNameIndex::<T>::get(&who).ok_or(Error::<T>::NotExists)?;
			let new_name =
				sp_std::str::from_utf8(&new_name[..]).map_err(|_| Error::<T>::InvalidNameFormat)?;
			let tag = T::TagGenerator::tag(&who, new_name)
				.map_err(|_| Error::<T>::TagGenerationFailed)?;
			let new_account_name =
				AccountName::new(&new_name, tag).map_err(|_| Error::<T>::InvalidNameFormat)?;
			AccountAliases::<T>::try_mutate(
				AccountAlias::AccountName(new_account_name),
				|maybe_value| -> DispatchResult {
					ensure!(maybe_value.is_none(), Error::<T>::InUse);
					*maybe_value = Some(who.clone());
					Ok(())
				},
			)?;
			AccountAliases::<T>::remove(AccountAlias::AccountName(account_name));
			AccountNameIndex::<T>::insert(&who, new_account_name);
			Self::deposit_event(Event::<T>::AccountNameUpdated {
				who,
				name: new_account_name,
				deleted: Some(account_name),
			});
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::connect_aliases())]
		pub fn connect_aliases(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::connect_aliases_secp256k1(&who)?;
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::force_set_account_name())]
		pub fn force_set_account_name(
			origin: OriginFor<T>,
			dest: AccountIdLookupOf<T>,
			name: Vec<u8>,
			tag: u16,
		) -> DispatchResult {
			ensure_root(origin)?;
			let dest = T::Lookup::lookup(dest)?;
			let name =
				sp_std::str::from_utf8(&name[..]).map_err(|_| Error::<T>::InvalidNameFormat)?;
			let new_account_name =
				AccountName::new(&name, tag).map_err(|_| Error::<T>::InvalidNameFormat)?;
			AccountAliases::<T>::try_mutate(
				AccountAlias::AccountName(new_account_name),
				|maybe_value| -> DispatchResult {
					ensure!(maybe_value.is_none(), Error::<T>::InUse);
					*maybe_value = Some(dest.clone());
					Ok(())
				},
			)?;

			let past_name = AccountNameIndex::<T>::get(&dest);
			match past_name {
				Some(past_name) => {
					AccountAliases::<T>::remove(AccountAlias::AccountName(past_name))
				},
				None => (),
			};
			AccountNameIndex::<T>::insert(&dest, new_account_name);
			Self::deposit_event(Event::<T>::AccountNameUpdated {
				who: dest,
				name: new_account_name,
				deleted: past_name,
			});
			Ok(())
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account name was updated.
		AccountNameUpdated { who: T::AccountId, name: AccountName, deleted: Option<AccountName> },
		/// An ethereum address was published.
		EthereumAddressPublished { who: T::AccountId, address: [u8; 20] },
		/// An cosmos address was published.
		CosmosAddressPublished { who: T::AccountId, address: [u8; 20] },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The account name already exists.
		AlreadyExists,
		/// The account name does not exists.
		NotExists,
		/// The account name is not available.
		InUse,
		/// Invalid name foramt.
		InvalidNameFormat,
		/// Tag generation failed.
		TagGenerationFailed,
		/// Ethereum address conversion failed.
		EthereumAddressConversionFailed,
		/// Cosmos address conversion failed.
		CosmosAddressConversionFailed,
	}

	#[pallet::storage]
	pub type AccountAliases<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountAlias, T::AccountId>;
	#[pallet::storage]
	pub type AccountNameIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, AccountName>;
}

impl<T: Config> Pallet<T>
where
	T::AccountId: EcdsaExt,
{
	// PUBLIC IMMUTABLES

	/// Lookup an AccountAlias to get an Id, if exists.
	pub fn lookup(alias: &AccountAlias) -> Option<T::AccountId> {
		AccountAliases::<T>::get(alias).map(|x| x)
	}

	pub fn connect_aliases_secp256k1(who: &T::AccountId) -> Result<(), DispatchError> {
		let ethereum_address = who
			.to_eth_address()
			.map(|x| x.into())
			.ok_or(Error::<T>::EthereumAddressConversionFailed)?;
		if AccountAliases::<T>::get(AccountAlias::EthereumAddress(ethereum_address)).is_none() {
			AccountAliases::<T>::insert(AccountAlias::EthereumAddress(ethereum_address), who);
			Self::deposit_event(Event::<T>::EthereumAddressPublished {
				who: who.clone(),
				address: ethereum_address,
			});
		}
		let cosmos_address = who
			.to_cosm_address()
			.map(|x| x.into())
			.ok_or(Error::<T>::CosmosAddressConversionFailed)?;
		if AccountAliases::<T>::get(AccountAlias::CosmosAddress(cosmos_address)).is_none() {
			AccountAliases::<T>::insert(AccountAlias::CosmosAddress(cosmos_address), who);
			Self::deposit_event(Event::<T>::CosmosAddressPublished {
				who: who.clone(),
				address: cosmos_address,
			});
		}
		Ok(())
	}
}

impl<T: Config> StaticLookup for Pallet<T>
where
	T::AccountId: EcdsaExt,
{
	type Source = MultiAddress<T::AccountId, AccountName>;
	type Target = T::AccountId;

	fn lookup(a: Self::Source) -> Result<Self::Target, LookupError> {
		match a {
			MultiAddress::Id(id) => Ok(id),
			MultiAddress::Index(name) => {
				Self::lookup(&AccountAlias::AccountName(name)).ok_or(LookupError)
			},
			_ => Err(LookupError),
		}
	}

	fn unlookup(a: Self::Target) -> Self::Source {
		MultiAddress::Id(a)
	}
}
