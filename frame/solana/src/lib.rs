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
pub mod runtime;
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
	use crate::{
		runtime::bank::{Bank, TransactionSimulationResult},
		svm::{
			transaction_processor::{
				ExecutionRecordingConfig, LoadAndExecuteSanitizedTransactionOutput,
				TransactionProcessingConfig, TransactionProcessingEnvironment,
				TransactionProcessor,
			},
			transaction_results::TransactionExecutionResult,
		},
	};
	use core::marker::PhantomData;
	use frame_support::{
		dispatch::DispatchInfo,
		pallet_prelude::*,
		traits::{
			fungible,
			fungible::Inspect,
			tokens::{Fortitude::Polite, Preservation::Preserve},
		},
		BoundedBTreeSet,
	};
	use frame_system::{pallet_prelude::*, CheckWeight};
	use nostd::sync::Arc;
	use np_runtime::traits::LossyInto;
	use parity_scale_codec::Codec;
	use solana_sdk::{
		account::Account,
		bpf_loader, clock,
		feature_set::FeatureSet,
		fee_calculator::FeeCalculator,
		hash::Hash,
		message::SimpleAddressLoader,
		reserved_account_keys::ReservedAccountKeys,
		transaction::{MessageHash, SanitizedTransaction},
		transaction_context::TransactionAccount,
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
		type AccountIdConversion: ConvertBack<Pubkey, Self::AccountId>;

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

		#[pallet::constant]
		type ScanResultsLimitBytes: Get<Option<u32>>;

		#[pallet::constant]
		type TransactionCacheLimit: Get<u32>;
	}

	pub mod config_preludes {
		use super::*;
		use frame_support::{derive_impl, parameter_types, traits::ConstU64};

		/// A configuration for testing.
		pub struct TestDefaultConfig;

		#[derive_impl(frame_system::config_preludes::TestDefaultConfig, no_aggregated_types)]
		impl frame_system::DefaultConfig for TestDefaultConfig {}

		#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
		impl pallet_timestamp::DefaultConfig for TestDefaultConfig {}

		parameter_types! {
			pub const ScanResultsLimitBytes: Option<u32> = None;
		}

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
			/// Maximum scan result size in bytes.
			type ScanResultsLimitBytes = ScanResultsLimitBytes;

			type TransactionCacheLimit = ConstU32<10000>;
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::origin]
	pub type Origin = RawOrigin;

	#[pallet::error]
	pub enum Error<T> {
		/// Failed to reallocate account data of this length
		InvalidRealloc,
		///
		CacheLimitReached,
	}

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

	#[pallet::storage]
	#[pallet::getter(fn transaction_count)]
	pub type TransactionCount<T> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	pub type TransactionCache<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::Hash,
		BoundedBTreeSet<T::Hash, T::TransactionCacheLimit>,
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
			let blockhash = <frame_system::Pallet<T>>::block_hash(to_remove);
			<BlockhashQueue<T>>::remove(blockhash);
			<TransactionCache<T>>::remove(blockhash);

			let count = frame_system::Pallet::<T>::extrinsic_count() as u64;
			TransactionCount::<T>::mutate(|total_count| {
				*total_count = total_count.saturating_add(count)
			});
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
		pub fn create_account(pubkey: Pubkey, owner: Pubkey, executable: bool) {
			let who = T::AccountIdConversion::convert(pubkey);
			<frame_system::Pallet<T>>::inc_sufficients(&who);
			<AccountMeta<T>>::insert(
				who,
				AccountMetadata { owner, executable, rent_epoch: u64::MAX },
			);
		}

		pub fn deploy_program(
			pubkey: Pubkey,
			data: Vec<u8>,
			owner: Option<Pubkey>,
		) -> Result<(), Error<T>> {
			let program_id = T::AccountIdConversion::convert(pubkey);
			let owner = owner.unwrap_or(bpf_loader::id());
			Self::create_account(pubkey, owner, true);
			if !data.is_empty() {
				<AccountData<T>>::insert(
					program_id,
					BoundedVec::try_from(data.to_vec()).map_err(|_| Error::InvalidRealloc)?,
				);
			}
			Ok(())
		}

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

			if bank.load_execute_and_commit_sanitized_transaction(sanitized_tx.clone()).is_ok() {
				Self::update_transaction_cache(&sanitized_tx)?;
			};

			Ok(().into())
		}

		// TODO: unimplemented.
		fn validate_transaction_in_pool(
			_fee_payer: Pubkey,
			transaction: &Transaction,
		) -> TransactionValidity {
			let mut builder = ValidTransactionBuilder::default();

			Self::check_transaction(&transaction)?;

			builder.build()
		}

		// TODO: unimplemented.
		fn validate_transaction_in_block(
			_fee_payer: Pubkey,
			transaction: &Transaction,
		) -> Result<(), TransactionValidityError> {
			Self::check_transaction(&transaction)?;

			Ok(())
		}

		pub fn get_balance(pubkey: Pubkey) -> u64 {
			Lamports::<T>::new(T::Currency::reducible_balance(
				&T::AccountIdConversion::convert(pubkey),
				Preserve,
				Polite,
			))
			.get()
		}

		pub fn get_account_info(pubkey: Pubkey) -> Option<Account> {
			let meta = AccountMeta::<T>::get(T::AccountIdConversion::convert(pubkey));

			if let Some(meta) = meta {
				let lamports = Pallet::<T>::get_balance(pubkey);
				let data: Vec<u8> =
					AccountData::<T>::get(T::AccountIdConversion::convert(pubkey)).into();

				Some(Account {
					lamports,
					data,
					owner: meta.owner,
					executable: meta.executable,
					rent_epoch: meta.rent_epoch,
				})
			} else {
				None
			}
		}

		pub fn get_transaction_count() -> u64 {
			TransactionCount::<T>::get()
		}

		pub fn simulate_transaction(
			sanitized_tx: SanitizedTransaction,
			enable_cpi_recording: bool,
		) -> TransactionSimulationResult {
			let account_keys = sanitized_tx.message().account_keys();
			let number_of_accounts = account_keys.len();

			let bank = <Bank<T>>::new(<Slot<T>>::get());

			let check_result =
				bank.check_transaction(&sanitized_tx, T::BlockhashQueueMaxAge::get());

			let blockhash =
				T::HashConversion::convert_back(<frame_system::Pallet<T>>::parent_hash());
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
					enable_cpi_recording,
					enable_log_recording: true,
					enable_return_data_recording: true,
				},
				transaction_account_lock_limit: None,
			};

			let transaction_processor = TransactionProcessor::default();
			let LoadAndExecuteSanitizedTransactionOutput {
				loaded_transaction,
				execution_result,
				..
			} = transaction_processor.load_and_execute_sanitized_transaction(
				&bank,
				&sanitized_tx,
				check_result,
				&processing_environment,
				&processing_config,
			);

			let post_simulation_accounts = loaded_transaction
				.ok()
				.map(|transaction| {
					transaction
						.accounts
						.into_iter()
						.take(number_of_accounts)
						.map(|(pubkey, account)| (pubkey, Account::from(account).into()))
						.collect::<Vec<TransactionAccount>>()
				})
				.unwrap_or_default();

			let flattened_result = execution_result.flattened_result();
			let (logs, return_data, inner_instructions) = match execution_result {
				TransactionExecutionResult::Executed { details, .. } =>
					(details.log_messages, details.return_data, details.inner_instructions),
				TransactionExecutionResult::NotExecuted(_) => (None, None, None),
			};
			let logs = logs.unwrap_or_default();

			// TODO: Calculate units_consumed
			TransactionSimulationResult {
				result: flattened_result,
				logs,
				post_simulation_accounts,
				units_consumed: 0,
				return_data,
				inner_instructions,
			}
		}

		fn update_transaction_cache(sanitized_tx: &SanitizedTransaction) -> Result<(), Error<T>> {
			let blockhash = T::HashConversion::convert(*sanitized_tx.message().recent_blockhash());
			let message_hash = T::HashConversion::convert(*sanitized_tx.message_hash());

			ensure!(
				<TransactionCache<T>>::get(blockhash).len() <
					T::TransactionCacheLimit::get() as usize,
				Error::<T>::CacheLimitReached
			);
			<TransactionCache<T>>::mutate(blockhash, |cache| {
				cache.try_insert(message_hash).unwrap()
			});

			Ok(())
		}

		pub(crate) fn check_transaction(
			transaction: &Transaction,
		) -> Result<(), InvalidTransaction> {
			// TODO: Update error code.
			let sanitized_tx = SanitizedTransaction::try_create(
				transaction.clone(),
				MessageHash::Compute,
				None,
				SimpleAddressLoader::Disabled,
				&ReservedAccountKeys::empty_key_set(),
			)
			.map_err(|_| InvalidTransaction::Custom(0))?;

			if Self::is_transaction_already_processed(&sanitized_tx) {
				return Err(InvalidTransaction::Custom(0));
			}

			Ok(())
		}

		fn is_transaction_already_processed(sanitized_tx: &SanitizedTransaction) -> bool {
			let blockhash = T::HashConversion::convert(*sanitized_tx.message().recent_blockhash());
			let message_hash = T::HashConversion::convert(*sanitized_tx.message_hash());

			<TransactionCache<T>>::get(blockhash).contains(&message_hash)
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
