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
use frame_support::traits::{Currency, ReservableCurrency};
use np_runtime::AccountName;
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{LookupError, StaticLookup},
	MultiAddress,
};
use sp_std::prelude::*;

/// A provider for tag number that discriminates the same name accounts.
pub trait TagProvider<T: Config> {
	fn tag(id: &T::AccountId, name: &str) -> Result<u16, ()>;
}

/// A generator for ethereum address.
pub trait EthAddressGenerator<T: Config> {
	fn generate(id: &T::AccountId) -> Result<[u8; 20], ()>;
}

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// The deposit needed for reserving an index.
		#[pallet::constant]
		type Deposit: Get<BalanceOf<Self>>;
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
		/// The provider for tag number that discriminates the same name accounts.
		type TagProvider: TagProvider<Self>;
		/// The generator for ethereum address.
		type EthAddressGenerator: EthAddressGenerator<Self>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::claim_account_name())]
		pub fn claim_account_name(origin: OriginFor<T>, name: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(AccountNamesIndex::<T>::get(&who).is_none(), Error::<T>::AlreadyAliased);
			let name =
				sp_std::str::from_utf8(&name[..]).map_err(|_| Error::<T>::InvalidNameFormat)?;
			let tag =
				T::TagProvider::tag(&who, name).map_err(|_| Error::<T>::FailedTagGeneration)?;
			let account_name =
				AccountName::new(&name, tag).map_err(|_| Error::<T>::InvalidNameFormat)?;
			AccountAliases::<T>::try_mutate(
				AccountAlias::AccountName(account_name),
				|maybe_value| -> DispatchResult {
					ensure!(maybe_value.is_none(), Error::<T>::InUse);
					*maybe_value = Some((who.clone(), T::Deposit::get()));
					T::Currency::reserve(&who, T::Deposit::get())?;
					Ok(())
				},
			)?;
			AccountNamesIndex::<T>::insert(&who, account_name);
			Self::deposit_event(Event::<T>::AccountNameAliased { who, aliased: account_name });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::reclaim_account_name())]
		pub fn reclaim_account_name(origin: OriginFor<T>, new_name: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let account_name = AccountNamesIndex::<T>::get(&who).ok_or(Error::<T>::NotAliased)?;
			let new_name =
				sp_std::str::from_utf8(&new_name[..]).map_err(|_| Error::<T>::InvalidNameFormat)?;
			let tag =
				T::TagProvider::tag(&who, new_name).map_err(|_| Error::<T>::FailedTagGeneration)?;
			let new_account_name =
				AccountName::new(&new_name, tag).map_err(|_| Error::<T>::InvalidNameFormat)?;
			AccountAliases::<T>::try_mutate(
				AccountAlias::AccountName(new_account_name),
				|maybe_value| -> DispatchResult {
					ensure!(maybe_value.is_none(), Error::<T>::InUse);
					*maybe_value = Some((who.clone(), T::Deposit::get()));
					Ok(())
				},
			)?;
			AccountAliases::<T>::remove(AccountAlias::AccountName(account_name));
			AccountNamesIndex::<T>::insert(&who, new_account_name);
			Self::deposit_event(Event::<T>::AccountNameRealiased {
				who,
				past_aliased: account_name,
				current_aliased: new_account_name,
			});
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::claim_k1_address())]
		pub fn claim_k1_address(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let ethereum_address = T::EthAddressGenerator::generate(&who)
				.map_err(|_| Error::<T>::FailedEthAddressGeneration)?;
			AccountAliases::<T>::try_mutate(
				AccountAlias::EthereumAddress(ethereum_address),
				|maybe_value| -> DispatchResult {
					ensure!(maybe_value.is_none(), Error::<T>::InUse);
					*maybe_value = Some((who.clone(), T::Deposit::get()));
					Ok(())
				},
			)?;
			Self::deposit_event(Event::<T>::EthAddressAliased { who, aliased: ethereum_address });
			Ok(())
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account name was aliased.
		AccountNameAliased { who: T::AccountId, aliased: AccountName },
		/// An account name was realiased.
		AccountNameRealiased {
			who: T::AccountId,
			past_aliased: AccountName,
			current_aliased: AccountName,
		},
		/// An ethereum address was aliased.
		EthAddressAliased { who: T::AccountId, aliased: [u8; 20] },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The account was already aliased.
		AlreadyAliased,
		/// The account was not aliased.
		NotAliased,
		/// The account name was not available.
		InUse,
		/// Invalid name foramt.
		InvalidNameFormat,
		/// Failed to generate tag.
		FailedTagGeneration,
		/// Failed to generate ethereum address.
		FailedEthAddressGeneration,
	}

	#[pallet::storage]
	pub type AccountAliases<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountAlias, (T::AccountId, BalanceOf<T>)>;
	#[pallet::storage]
	pub type AccountNamesIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, AccountName>;
}

impl<T: Config> Pallet<T> {
	// PUBLIC IMMUTABLES

	/// Lookup an AccountName to get an Id, if there's one there.
	pub fn lookup_name(name: AccountName) -> Option<T::AccountId> {
		AccountAliases::<T>::get(AccountAlias::AccountName(name)).map(|x| x.0)
	}

	/// Lookup an address to get an Id, if there's one there.
	pub fn lookup_address(a: MultiAddress<T::AccountId, AccountName>) -> Option<T::AccountId> {
		match a {
			MultiAddress::Id(i) => Some(i),
			MultiAddress::Index(i) => Self::lookup_name(i),
			_ => None,
		}
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
