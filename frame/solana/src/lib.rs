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

#[macro_use]
extern crate derive_where;

pub use pallet::*;

pub use crate::runtime::invoke_context;
pub use solana_rbpf;
pub use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction as Transaction};

#[cfg(test)]
mod mock;
mod programs;
mod runtime;
#[cfg(test)]
mod tests;

use frame_support::{
	dispatch::{DispatchErrorWithPostInfo, PostDispatchInfo},
	sp_runtime::{self, RuntimeDebug},
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

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::runtime::{
		account::AccountSharedData,
		bank::Bank,
		transaction_processing_callback::TransactionProcessingCallback,
		transaction_processor::{
			ExecutionRecordingConfig, TransactionProcessingConfig,
			TransactionProcessingEnvironment, TransactionProcessor,
		},
	};
	use core::fmt::Debug;
	use frame_support::{dispatch::DispatchInfo, pallet_prelude::*, traits::fungible};
	use frame_system::{pallet_prelude::*, CheckWeight};
	use nostd::sync::Arc;
	use np_runtime::traits::LossyInto;
	use parity_scale_codec::Codec;
	use solana_sdk::{
		feature_set::FeatureSet,
		fee_calculator::FeeCalculator,
		hash::Hash,
		message::SimpleAddressLoader,
		reserved_account_keys::ReservedAccountKeys,
		transaction::{MessageHash, SanitizedTransaction},
	};
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, Convert, DispatchInfoOf, Dispatchable, One, Saturating},
		transaction_validity::{
			InvalidTransaction, TransactionValidity, TransactionValidityError,
			ValidTransactionBuilder,
		},
		FixedPointOperand,
	};

	#[pallet::config(with_default)]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		#[pallet::no_default]
		type AccountIdConversion: Convert<Pubkey, Self::AccountId>;

		#[pallet::no_default]
		type HashConversion: Convert<Self::Hash, Hash>;

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
	}

	pub mod config_preludes {
		use super::*;
		use frame_support::{derive_impl, traits::ConstU64};

		/// A configuration for testing.
		pub struct TestDefaultConfig;

		#[derive_impl(frame_system::config_preludes::TestDefaultConfig, no_aggregated_types)]
		impl frame_system::DefaultConfig for TestDefaultConfig {}

		#[frame_support::register_default_impl(TestDefaultConfig)]
		impl DefaultConfig for TestDefaultConfig {
			type DecimalMultiplier = ConstU64<1>;
			/// Hashes older than 2 minutes (20 blocks) will be dropped from the blockhash queue.
			type BlockhashQueueMaxAge = ConstU64<20>;
			/// Maximum permitted size of account data (10 MiB).
			type MaxPermittedDataLength = ConstU32<{ 10 * 1024 * 1024 }>;
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::origin]
	pub type Origin = RawOrigin;

	#[derive(Decode, Encode, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct HashInfo<T: Config> {
		fee_calculator: FeeCalculator,
		hash_index: BlockNumberFor<T>,
		timestamp: T::Moment,
	}

	impl<T: Config> HashInfo<T> {
		pub fn lamports_per_signature(&self) -> u64 {
			self.fee_calculator.lamports_per_signature
		}
	}

	/// FIFO queue of `recent_blockhashes` item to verify nonces.
	#[pallet::storage]
	#[pallet::getter(fn blockhash_queue)]
	pub type BlockhashQueue<T: Config> = StorageMap<_, Twox64Concat, T::Hash, HashInfo<T>>;

	// AccountRentState?

	#[pallet::storage]
	#[pallet::getter(fn account_meta)]
	pub type AccountMeta<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, crate::runtime::meta::AccountMeta>;

	#[pallet::storage]
	#[pallet::getter(fn account_data)]
	pub type AccountData<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<u8, T::MaxPermittedDataLength>>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
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
			<BlockhashQueue<T>>::remove(&<frame_system::Pallet<T>>::block_hash(to_remove));
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

			let bank = <Bank<T>>::new();

			let check_result =
				bank.check_transaction(&sanitized_tx, T::BlockhashQueueMaxAge::get());

			let blockhash = T::HashConversion::convert(<frame_system::Pallet<T>>::parent_hash());
			// FIXME: Update lamports_per_signature.
			let lamports_per_signature = Default::default();
			let processing_environment = TransactionProcessingEnvironment {
				blockhash,
				epoch_total_stake: None,
				epoch_vote_accounts: None,
				feature_set: Arc::new(FeatureSet::default()),
				fee_structure: None,
				lamports_per_signature,
				rent_collector: None,
			};
			// FIXME: Update fields.
			let processing_config = TransactionProcessingConfig {
				account_overrides: None,
				check_program_modification_slot: false,
				compute_budget: None,
				log_messages_bytes_limit: None,
				limit_to_load_programs: false,
				recording_config: ExecutionRecordingConfig {
					enable_cpi_recording: false,
					enable_log_recording: true,
					enable_return_data_recording: true,
				},
				transaction_account_lock_limit: None,
			};

			let transaction_processor = TransactionProcessor::default();
			let sanitized_output = transaction_processor.load_and_execute_sanitized_transaction(
				&bank,
				sanitized_tx,
				check_result,
				&processing_environment,
				&processing_config,
			);

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
						fee_payer: sanitized_tx.message().fee_payer().clone(),
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
