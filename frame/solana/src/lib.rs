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
#![allow(unexpected_cfgs, unused)]
#![allow(clippy::too_many_arguments)]

extern crate alloc;

#[macro_use]
extern crate derive_where;

#[cfg_attr(feature = "std", macro_use)]
#[cfg(feature = "std")]
extern crate solana_metrics;

pub use pallet::*;
pub use types::*;

pub use solana_rbpf;
pub use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction as Transaction};

#[cfg(test)]
mod mock;
mod runtime;
mod svm;
#[cfg(test)]
mod tests;
mod types;

use frame_support::{
	dispatch::{DispatchErrorWithPostInfo, PostDispatchInfo},
	sp_runtime::{self, RuntimeDebug, SaturatedConversion},
	traits::EnsureOrigin,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

pub type BalanceOf<T> = <T as Config>::Balance;

#[derive(Clone, Eq, PartialEq, RuntimeDebug, Decode, Encode, MaxEncodedLen, TypeInfo)]
pub enum RawOrigin {
	SolanaTransaction(Pubkey),
}

pub fn ensure_solana_transaction<OuterOrigin>(o: OuterOrigin) -> Result<Pubkey, &'static str>
where
	OuterOrigin: Into<Result<RawOrigin, OuterOrigin>>,
{
	match o.into() {
		Ok(RawOrigin::SolanaTransaction(n)) => Ok(n),
		_ => Err("bad origin: expected to be an Solana transaction"),
	}
}

pub struct EnsureSolanaTransaction;
impl<O: Into<Result<RawOrigin, O>> + From<RawOrigin>> EnsureOrigin<O> for EnsureSolanaTransaction {
	type Success = Pubkey;
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().map(|o| match o {
			RawOrigin::SolanaTransaction(id) => id,
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<O, ()> {
		Ok(O::from(RawOrigin::SolanaTransaction(Default::default())))
	}
}

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use crate::runtime::bank::Bank;
	use core::marker::PhantomData;
	use frame_support::{dispatch::DispatchInfo, pallet_prelude::*, traits::fungible};
	use frame_system::{pallet_prelude::*, CheckWeight};
	use np_runtime::traits::LossyInto;
	use solana_sdk::{
		account::Account,
		clock,
		fee_calculator::FeeCalculator,
		hash::Hash,
		message::SimpleAddressLoader,
		reserved_account_keys::ReservedAccountKeys,
		transaction::{MessageHash, SanitizedTransaction},
	};
	use sp_runtime::{
		traits::{
			AtLeast32BitUnsigned, Convert, ConvertBack, DispatchInfoOf, Dispatchable, One,
			Saturating,
		},
		transaction_validity::{
			InvalidTransaction, TransactionValidity, TransactionValidityError,
			ValidTransactionBuilder,
		},
	};

	#[pallet::config(with_default)]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		#[pallet::no_default]
		type AccountIdConversion: Convert<Pubkey, Self::AccountId>;

		#[pallet::no_default]
		type HashConversion: ConvertBack<Hash, Self::Hash>;

		#[pallet::no_default]
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ TypeInfo
			+ From<u64>
			+ LossyInto<u64>;

		#[pallet::no_default]
		type Currency: fungible::Mutate<Self::AccountId, Balance = Self::Balance>;

		#[pallet::constant]
		#[pallet::no_default_bounds]
		type DecimalMultiplier: Get<BalanceOf<Self>>;

		/// The maximum age for entries in the blockhash queue.
		///
		/// WARN: This value should less than `frame_system::Config::BlockHashCount`.
		#[pallet::constant]
		#[pallet::no_default_bounds]
		type BlockhashQueueMaxAge: Get<BlockNumberFor<Self>>;

		/// Maximum permitted size of account data (10 MiB).
		#[pallet::constant]
		type MaxPermittedDataLength: Get<u32>;

		/// Timestamp at genesis block.
		#[pallet::constant]
		#[pallet::no_default_bounds]
		type GenesisTimestamp: Get<Self::Moment>;
	}

	pub mod config_preludes {
		use super::*;
		use frame_support::{derive_impl, traits::ConstU64};

		/// A configuration for testing.
		pub struct TestDefaultConfig;

		#[derive_impl(frame_system::config_preludes::TestDefaultConfig, no_aggregated_types)]
		impl frame_system::DefaultConfig for TestDefaultConfig {}

		#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
		impl pallet_timestamp::DefaultConfig for TestDefaultConfig {}

		#[frame_support::register_default_impl(TestDefaultConfig)]
		impl DefaultConfig for TestDefaultConfig {
			type DecimalMultiplier = ConstU64<1>;
			/// Hashes older than 2 minutes (20 blocks) will be dropped from the blockhash queue.
			type BlockhashQueueMaxAge = ConstU64<20>;
			/// Maximum permitted size of account data (10 MiB).
			type MaxPermittedDataLength = ConstU32<{ 10 * 1024 * 1024 }>;
			/// Timestamp at genesis block (Solana).
			#[allow(clippy::inconsistent_digit_grouping)]
			type GenesisTimestamp = ConstU64<1584336540_000>;
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::origin]
	pub type Origin = RawOrigin;

	#[pallet::storage]
	#[pallet::getter(fn slot)]
	pub type Slot<T: Config> = StorageValue<_, clock::Slot, ValueQuery>;

	/// FIFO queue of `recent_blockhashes` item to verify nonces.
	#[pallet::storage]
	#[pallet::getter(fn blockhash_queue)]
	pub type BlockhashQueue<T: Config> = StorageMap<_, Twox64Concat, T::Hash, HashInfo<T>>;

	// AccountRentState?

	#[pallet::storage]
	#[pallet::getter(fn account_meta)]
	pub type AccountMeta<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, AccountMetadata>;

	#[pallet::storage]
	#[pallet::getter(fn account_data)]
	pub type AccountData<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<u8, T::MaxPermittedDataLength>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	#[derive_where(Default)]
	pub struct GenesisConfig<T: Config> {
		accounts: Vec<(Pubkey, Account)>,
		_marker: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			self.accounts.iter().for_each(|(pubkey, account)| {
				let who = T::AccountIdConversion::convert(*pubkey);
				assert!(<frame_system::Pallet<T>>::account_exists(&who));
				<AccountMeta<T>>::insert(
					&who,
					AccountMetadata {
						rent_epoch: account.rent_epoch,
						owner: account.owner,
						executable: account.executable,
					},
				);
				(!account.data.is_empty()).then(|| {
					<AccountData<T>>::insert(
						who,
						BoundedVec::try_from(account.data.clone()).expect("valid data"),
					);
				});
			});
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let elapsed =
				<pallet_timestamp::Now<T>>::get().saturating_sub(T::GenesisTimestamp::get());
			<Slot<T>>::put((elapsed / T::Moment::from(400u32)).saturated_into::<clock::Slot>());

			let parent_hash = <frame_system::Pallet<T>>::parent_hash();
			<BlockhashQueue<T>>::insert(
				parent_hash,
				HashInfo {
					// FIXME: Update fee calculator.
					fee_calculator: FeeCalculator::default(),
					hash_index: now.saturating_sub(One::one()),
					timestamp: <pallet_timestamp::Pallet<T>>::get(),
				},
			);
			Weight::zero()
		}

		fn on_finalize(now: BlockNumberFor<T>) {
			let max_age = T::BlockhashQueueMaxAge::get();
			let to_remove = now.saturating_sub(max_age).saturating_sub(One::one());
			<BlockhashQueue<T>>::remove(<frame_system::Pallet<T>>::block_hash(to_remove));
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(1_000)]
		pub fn transact(
			origin: OriginFor<T>,
			transaction: Transaction,
		) -> DispatchResultWithPostInfo {
			let pubkey = ensure_solana_transaction(origin)?;

			Self::apply_validated_transaction(pubkey, transaction)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_hash_info_if_valid(
			hash: &T::Hash,
			max_age: BlockNumberFor<T>,
		) -> Option<HashInfo<T>> {
			let last_hash_index =
				<frame_system::Pallet<T>>::block_number().saturating_sub(One::one());
			<BlockhashQueue<T>>::get(hash)
				.filter(|info| last_hash_index - info.hash_index <= max_age)
		}

		fn apply_validated_transaction(
			fee_payer: Pubkey,
			transaction: Transaction,
		) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo> {
			let sanitized_tx = SanitizedTransaction::try_create(
				transaction,
				MessageHash::Compute,
				None,
				SimpleAddressLoader::Disabled,
				&ReservedAccountKeys::empty_key_set(),
			)
			.expect("valid transaction");

			let bank = <Bank<T>>::new(<Slot<T>>::get());

			bank.load_execute_and_commit_sanitized_transaction(sanitized_tx);

			Ok(().into())
		}

		// TODO: unimplemented.
		fn validate_transaction_in_pool(
			_fee_payer: Pubkey,
			_transaction: &Transaction,
		) -> TransactionValidity {
			let mut builder = ValidTransactionBuilder::default();

			builder.build()
		}

		// TODO: unimplemented.
		fn validate_transaction_in_block(
			_fee_payer: Pubkey,
			_transaction: &Transaction,
		) -> Result<(), TransactionValidityError> {
			Ok(())
		}
	}

	// TODO: Generalize and move to higher level.
	pub struct SignedInfo {
		pub fee_payer: Pubkey,
		pub sanitized_tx: SanitizedTransaction,
	}

	impl<T> Call<T>
	where
		T: Config + Send + Sync,
		T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
		OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	{
		pub fn is_self_contained(&self) -> bool {
			matches!(self, Call::transact { .. })
		}

		pub fn check_self_contained(&self) -> Option<Result<SignedInfo, TransactionValidityError>> {
			if let Call::transact { transaction } = self {
				let sanitized_tx = match SanitizedTransaction::try_create(
					transaction.clone(),
					MessageHash::Compute,
					None,
					SimpleAddressLoader::Disabled,
					&ReservedAccountKeys::empty_key_set(),
				) {
					Ok(tx) => tx,
					// TODO: Update error code.
					Err(_) => return Some(Err(InvalidTransaction::Custom(0).into())),
				};
				match sanitized_tx.verify() {
					Ok(_) => Some(Ok(SignedInfo {
						fee_payer: *sanitized_tx.message().fee_payer(),
						sanitized_tx,
					})),
					Err(_) => Some(Err(InvalidTransaction::BadProof.into())),
				}
			} else {
				None
			}
		}

		pub fn pre_dispatch_self_contained(
			&self,
			origin: &SignedInfo,
			dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
			len: usize,
		) -> Option<Result<(), TransactionValidityError>> {
			if let Call::transact { transaction } = self {
				if let Err(e) = CheckWeight::<T>::do_pre_dispatch(dispatch_info, len) {
					return Some(Err(e));
				}

				Some(Pallet::<T>::validate_transaction_in_block(origin.fee_payer, transaction))
			} else {
				None
			}
		}

		pub fn validate_self_contained(
			&self,
			origin: &SignedInfo,
			dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
			len: usize,
		) -> Option<TransactionValidity> {
			if let Call::transact { transaction } = self {
				if let Err(e) = CheckWeight::<T>::do_validate(dispatch_info, len) {
					return Some(Err(e));
				}

				Some(Pallet::<T>::validate_transaction_in_pool(origin.fee_payer, transaction))
			} else {
				None
			}
		}
	}
}
