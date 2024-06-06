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

//! # Alias Pallet

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;

pub use pallet::*;

use crate::weights::WeightInfo;
use np_crypto::ecdsa::EcdsaExt;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{BoundedBTreeSet, DispatchError};
use sp_std::{collections::btree_set::BTreeSet, prelude::*};

#[cfg_attr(feature = "std", derive(Hash))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum AccountAlias {
	EthereumAddress([u8; 20]),
	CosmosAddress([u8; 20]),
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	/// The module's config trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An ethereum address was published.
		EthereumAddressPublished { who: T::AccountId, address: [u8; 20] },
		/// An cosmos address was published.
		CosmosAddressPublished { who: T::AccountId, address: [u8; 20] },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Ethereum address conversion failed.
		EthereumAddressConversionFailed,
		/// Cosmos address conversion failed.
		CosmosAddressConversionFailed,
	}

	#[pallet::storage]
	#[pallet::getter(fn accountid)]
	pub type AccountIdOf<T: Config> = StorageMap<_, Blake2_128Concat, AccountAlias, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn aliases)]
	pub type AccountAliases<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BoundedBTreeSet<AccountAlias, ConstU32<2>>>;
}

impl<T: Config> Pallet<T>
where
	T::AccountId: EcdsaExt,
{
	/// Lookup an AccountAlias to get an Id, if exists.
	pub fn lookup(alias: &AccountAlias) -> Option<T::AccountId> {
		AccountIdOf::<T>::get(alias).map(|x| x)
	}

	pub fn alias_secp256k1(who: &T::AccountId) -> Result<(), DispatchError> {
		let mut aliases = BTreeSet::new();
		let eth = who
			.to_eth_address()
			.map(|x| x.into())
			.ok_or(Error::<T>::EthereumAddressConversionFailed)?;
		let eth_alias = AccountAlias::EthereumAddress(eth);
		if AccountIdOf::<T>::get(eth_alias).is_none() {
			AccountIdOf::<T>::insert(eth_alias, who.clone());
			aliases.insert(eth_alias);
			Self::deposit_event(Event::<T>::EthereumAddressPublished {
				who: who.clone(),
				address: eth,
			});
		}
		let cosm = who
			.to_cosm_address()
			.map(|x| x.into())
			.ok_or(Error::<T>::CosmosAddressConversionFailed)?;
		let cosm_alias = AccountAlias::CosmosAddress(cosm);
		if AccountIdOf::<T>::get(cosm_alias).is_none() {
			AccountIdOf::<T>::insert(cosm_alias, who.clone());
			aliases.insert(cosm_alias);
			Self::deposit_event(Event::<T>::CosmosAddressPublished {
				who: who.clone(),
				address: cosm,
			});
		}
		if !aliases.is_empty() {
			AccountAliases::<T>::insert(
				who,
				BoundedBTreeSet::try_from(aliases)
					.map_err(|_| DispatchError::Other("Too many aliases"))?,
			);
		}
		Ok(())
	}
}
