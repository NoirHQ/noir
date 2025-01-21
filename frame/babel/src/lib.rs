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

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "cosmos")]
pub mod cosmos;
#[cfg(feature = "ethereum")]
pub mod ethereum;
pub mod extensions;
pub mod traits;

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
		traits::{
			fungible::Mutate as _,
			fungibles::Mutate,
			tokens::Preservation::{Expendable, Preserve},
		},
	};
	use frame_system::{ensure_root, pallet_prelude::*};
	use np_multimap::{
		traits::{UniqueMap, UniqueMultimap},
		Error as MapError, UniqueMapAdapter, UniqueMultimapAdapter,
	};
	use pallet_cosmos::{
		types::{AssetIdOf, DenomOf},
		AddressMapping as _,
	};
	use pallet_cosmos_types::address::acc_address_from_bech32;
	use pallet_cosmos_x_auth_signing::sign_verifiable_tx::traits::SigVerifiableTx;
	use pallet_evm::{AddressMapping as _, FrameSystemAccountProvider};
	use solana_sdk::transaction::VersionedTransaction;
	use sp_core::{ecdsa, H256};
	#[cfg(feature = "nostr")]
	use sp_runtime::traits::AccountIdConversion;
	use sp_runtime::traits::{
		AtLeast32BitUnsigned, ConvertBack, One, Saturating, StaticLookup, UniqueSaturatedInto,
	};

	type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;
	type BalanceOf<T> = <T as Config>::Balance;

	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ pallet_assets::Config
		+ pallet_balances::Config
		+ pallet_cosmos::Config<AssetId = <Self as pallet_assets::Config>::AssetId>
		+ pallet_ethereum::Config
		+ pallet_evm::Config<AccountProvider = FrameSystemAccountProvider<Self>>
		+ pallet_solana::Config
	{
		type AddressMap: UniqueMultimap<Self::AccountId, VarAddress>;
		type AssetMap: UniqueMap<AssetIdOf<Self>, DenomOf<Self>>;
		type Balance: Member
			+ Parameter
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ TypeInfo
			+ Into<<Self as pallet_balances::Config>::Balance>
			+ Into<<Self as pallet_assets::Config>::Balance>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {
		InvalidOrigin,
		InvalidTransaction,
	}

	/// Mapping from addresses to accounts.
	pub type AddressMap<T> = UniqueMultimapAdapter<
		<T as frame_system::Config>::AccountId,
		VarAddress,
		AddressMapStorage<T>,
		AddressIndex<T>,
		ConstU32<{ VarAddress::variant_count() }>,
		MapError,
	>;
	#[pallet::storage]
	pub type AddressMapStorage<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedBTreeSet<VarAddress, ConstU32<{ VarAddress::variant_count() }>>,
		ValueQuery,
	>;
	#[pallet::storage]
	pub type AddressIndex<T: Config> = StorageMap<_, Twox64Concat, VarAddress, T::AccountId>;

	/// Mapping from asset IDs to denoms.
	pub type AssetMap<T> =
		UniqueMapAdapter<AssetIdOf<T>, DenomOf<T>, AssetMapStorage<T>, AssetIndex<T>, MapError>;
	#[pallet::storage]
	pub type AssetMapStorage<T: Config> = StorageMap<_, Twox64Concat, AssetIdOf<T>, DenomOf<T>>;
	#[pallet::storage]
	pub type AssetIndex<T: Config> = StorageMap<_, Twox64Concat, DenomOf<T>, AssetIdOf<T>>;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		OriginFor<T>: Into<Result<pallet_ethereum::RawOrigin, OriginFor<T>>>
			+ Into<Result<pallet_cosmos::RawOrigin, OriginFor<T>>>
			+ Into<Result<pallet_solana::RawOrigin, OriginFor<T>>>,
		T::AccountId: TryInto<ecdsa::Public> + From<H256>,
		T::RuntimeOrigin: From<pallet_ethereum::RawOrigin>
			+ From<pallet_cosmos::RawOrigin>
			+ From<pallet_solana::RawOrigin>,
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
			let address = T::AddressMap::get(&who)
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

			// CheckNonce signed extension already increased the nonce at this point,
			// but EVM will increase it again, so we need to decrease it here.
			frame_system::Account::<T>::mutate(who, |account| {
				account.nonce = account.nonce.saturating_sub(T::Nonce::one());
			});
			pallet_ethereum::Pallet::<T>::transact(origin, transaction)
		}

		#[pallet::call_index(1)]
		#[pallet::weight({
			use cosmos_sdk_proto::traits::Message;
			use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx;
			use pallet_cosmos::weights::WeightInfo;
			use sp_runtime::traits::ConvertBack;

			Tx::decode(&mut &tx_bytes[..])
				.ok()
				.and_then(|tx| tx.auth_info)
				.and_then(|auth_info| auth_info.fee)
				.map_or(<T as pallet_cosmos::Config>::WeightInfo::base_weight(), |fee| {
					<T as pallet_cosmos::Config>::WeightToGas::convert_back(fee.gas_limit)
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

		// TODO: Need to adjust the call_index and weight
		#[pallet::call_index(4)]
		#[pallet::weight({ 1_000 })]
		pub fn solana_transact(
			origin: OriginFor<T>,
			transaction: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let pubkey = <T as pallet_solana::Config>::AccountIdConversion::convert_back(who);

			let transaction: VersionedTransaction =
				bincode::deserialize(&transaction).map_err(|_| Error::<T>::InvalidTransaction)?;

			let origin =
				T::RuntimeOrigin::from(pallet_solana::RawOrigin::SolanaTransaction(pubkey));

			pallet_solana::Pallet::<T>::transact(origin, transaction)
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
			use pallet_assets::weights::WeightInfo as _;
			use pallet_balances::weights::WeightInfo as _;

			match id {
				Some(_) => <T as pallet_assets::Config>::WeightInfo::transfer(),
				None => <T as pallet_balances::Config>::WeightInfo::transfer_keep_alive()
			}
		})]
		pub fn transfer(
			origin: OriginFor<T>,
			id: Option<T::AssetIdParameter>,
			dest: VarAddress,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let dest: T::AccountId = match dest {
				VarAddress::Cosmos(address) =>
					<T as pallet_cosmos::Config>::AddressMapping::into_account_id(address.into()),
				VarAddress::Ethereum(address) =>
					<T as pallet_evm::Config>::AddressMapping::into_account_id(address.into()),
				VarAddress::Polkadot(address) => H256::from(<[u8; 32]>::from(address)).into(),
				#[cfg(feature = "nostr")]
				VarAddress::Nostr(ref address) =>
					T::AddressMap::find_key(&dest).unwrap_or(address.into_account_truncating()),
				VarAddress::Solana(address) => H256::from(address).into(),
				_ => unreachable!(),
			};

			match id {
				Some(id) => <pallet_assets::Pallet<T> as Mutate<T::AccountId>>::transfer(
					id.into(),
					&who,
					&dest,
					value.into(),
					Expendable,
				)
				.map(|_| ()),
				None => pallet_balances::Pallet::<T>::transfer(&who, &dest, value.into(), Preserve)
					.map(|_| ()),
			}
		}
	}
}
