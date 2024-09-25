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

extern crate alloc;

#[cfg(feature = "cosmos")]
pub mod cosmos;
#[cfg(feature = "ethereum")]
pub mod ethereum;
pub mod extensions;

pub use extensions::unify_account::UnifyAccount;
pub use np_babel::Address;

#[cfg(feature = "pallet")]
pub use pallet::*;

#[cfg(feature = "pallet")]
#[frame_support::pallet]
pub mod pallet {
	use alloc::vec::Vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_core::ecdsa;
	use sp_runtime::traits::UniqueSaturatedInto;

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_cosmos::Config + pallet_ethereum::Config
	{
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
		OriginFor<T>: Into<Result<pallet_ethereum::RawOrigin, OriginFor<T>>>,
		T::AccountId: TryInto<ecdsa::Public>,
		T::RuntimeOrigin: From<pallet_ethereum::RawOrigin>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight({
			use ethereum::EnvelopedDecodable;
			use pallet_ethereum::{Transaction, TransactionData};
			use pallet_evm::GasWeightMapping;
			let without_base_extrinsic_weight = true;
			match <Transaction as EnvelopedDecodable>::decode(&transaction) {
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
			let public: ecdsa::Public = who.try_into().map_err(|_| Error::<T>::InvalidOrigin)?;
			let address: np_babel::EthereumAddress = public.into();

			let origin = T::RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(
				address.into(),
			));
			let transaction = ethereum::EnvelopedDecodable::decode(&transaction)
				.map_err(|_| Error::<T>::InvalidTransaction)?;

			pallet_ethereum::Pallet::<T>::transact(origin, transaction)
		}
	}
}
