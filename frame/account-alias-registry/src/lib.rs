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
pub trait AccountNameTagGenerator<T: Config> {
	fn tag(id: &T::AccountId, name: &str) -> Result<u16, ()>;
}

/// A converter for ethereum address.
pub trait AccountIdToEthAddress<T: Config> {
	fn convert(id: &T::AccountId) -> Result<[u8; 20], ()>;
}

#[cfg_attr(feature = "std", derive(Hash))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum AccountAlias {
	AccountName(AccountName),
	EthereumAddress([u8; 20]),
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
		type AccountNameTagGenerator: AccountNameTagGenerator<Self>;
		/// The generator for ethereum address.
		type AccountIdToEthAddress: AccountIdToEthAddress<Self>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_account_name())]
		pub fn create_account_name(origin: OriginFor<T>, name: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(AccountNameIndex::<T>::get(&who).is_none(), Error::<T>::AlreadyAssigned);
			let name =
				sp_std::str::from_utf8(&name[..]).map_err(|_| Error::<T>::InvalidNameFormat)?;
			let tag = T::AccountNameTagGenerator::tag(&who, name)
				.map_err(|_| Error::<T>::TagGenerationFailed)?;
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
			Self::deposit_event(Event::<T>::AccountNameAssigned {
				who,
				name: account_name,
				unassigned: None,
			});
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_account_name())]
		pub fn update_account_name(origin: OriginFor<T>, new_name: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let account_name = AccountNameIndex::<T>::get(&who).ok_or(Error::<T>::NotAssigned)?;
			let new_name =
				sp_std::str::from_utf8(&new_name[..]).map_err(|_| Error::<T>::InvalidNameFormat)?;
			let tag = T::AccountNameTagGenerator::tag(&who, new_name)
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
			Self::deposit_event(Event::<T>::AccountNameAssigned {
				who,
				name: new_account_name,
				unassigned: Some(account_name),
			});
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::assign_all_available_aliases())]
		pub fn assign_all_available_aliases(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::assign_secp256k1_aliases(&who)?;
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::force_assign_account_name())]
		pub fn force_assign_account_name(
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
				Some(past_name) =>
					AccountAliases::<T>::remove(AccountAlias::AccountName(past_name)),
				None => (),
			};
			AccountNameIndex::<T>::insert(&dest, new_account_name);
			Self::deposit_event(Event::<T>::AccountNameAssigned {
				who: dest,
				name: new_account_name,
				unassigned: past_name,
			});
			Ok(())
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account name was assigned.
		AccountNameAssigned {
			who: T::AccountId,
			name: AccountName,
			unassigned: Option<AccountName>,
		},
		/// An ethereum address was assigned.
		EthAddressAssigned { who: T::AccountId, address: [u8; 20] },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The account was already assigned.
		AlreadyAssigned,
		/// The account was not assigned.
		NotAssigned,
		/// The account name was not available.
		InUse,
		/// Invalid name foramt.
		InvalidNameFormat,
		/// Tag generation failed.
		TagGenerationFailed,
		/// Ethereum address conversion failed.
		EthAddressConversionFailed,
	}

	#[pallet::storage]
	pub type AccountAliases<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountAlias, T::AccountId>;
	#[pallet::storage]
	pub type AccountNameIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, AccountName>;
}

impl<T: Config> Pallet<T> {
	// PUBLIC IMMUTABLES

	/// Lookup an AccountName to get an Id, if exists.
	pub fn lookup_name(name: AccountName) -> Option<T::AccountId> {
		AccountAliases::<T>::get(AccountAlias::AccountName(name)).map(|x| x)
	}

	/// Lookup an address to get an Id, if exists.
	pub fn lookup_address(a: MultiAddress<T::AccountId, AccountName>) -> Option<T::AccountId> {
		match a {
			MultiAddress::Id(i) => Some(i),
			MultiAddress::Index(i) => Self::lookup_name(i),
			_ => None,
		}
	}

	pub fn assign_secp256k1_aliases(who: &T::AccountId) -> Result<(), DispatchError> {
		let ethereum_address = T::AccountIdToEthAddress::convert(who)
			.map_err(|_| Error::<T>::EthAddressConversionFailed)?;
		AccountAliases::<T>::insert(AccountAlias::EthereumAddress(ethereum_address), who);
		Self::deposit_event(Event::<T>::EthAddressAssigned {
			who: who.clone(),
			address: ethereum_address,
		});
		Ok(())
	}
}

impl<T: Config> StaticLookup for Pallet<T> {
	type Source = MultiAddress<T::AccountId, AccountName>;
	type Target = T::AccountId;

	fn lookup(a: Self::Source) -> Result<Self::Target, LookupError> {
		Self::lookup_address(a).ok_or(LookupError)
	}

	fn unlookup(a: Self::Target) -> Self::Source {
		MultiAddress::Id(a)
	}
}
