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

//! Adapter types for Cosmos pallet compatibility.

use np_crypto::ecdsa::EcdsaExt;
use pallet_account_alias_registry::AccountAlias;
use pallet_cosmos::{AddressMapping, EnsureAddressOrigin};
use sp_core::{ecdsa, Hasher, H160, H256};
use sp_std::marker::PhantomData;

/// Hashed address mapping.
pub struct HashedAddressMapping<T, H>(PhantomData<T>, PhantomData<H>);

impl<T, H> AddressMapping<T::AccountId> for HashedAddressMapping<T, H>
where
	T: pallet_account_alias_registry::Config,
	T::AccountId: From<H256> + EcdsaExt,
	H: Hasher<Out = H256>,
{
	fn into_account_id(address: H160) -> T::AccountId {
		let alias = AccountAlias::CosmosAddress(address.into());
		if let Some(x) = pallet_account_alias_registry::Pallet::<T>::lookup(&alias) {
			return x
		}
		let mut data = [0u8; 24];
		data[0..4].copy_from_slice(b"cosm:");
		data[4..24].copy_from_slice(&address[..]);
		let hash = H::hash(&data);

		hash.into()
	}
}
