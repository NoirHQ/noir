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

use crate::{
	svm::{
		account_loader::{
			CheckedTransactionDetails, TransactionCheckResult, TransactionLoadResult,
		},
		nonce_info::{NonceInfo, NoncePartial},
		rollback_accounts::RollbackAccounts,
		transaction_processing_callback::TransactionProcessingCallback,
		transaction_processor::{
			ExecutionRecordingConfig, TransactionLogMessages, TransactionProcessingConfig,
			TransactionProcessingEnvironment, TransactionProcessor,
		},
		transaction_results::TransactionExecutionResult,
	},
	AccountData, AccountMeta, AccountMetadata, Config, Lamports, Pallet,
};
use frame_support::{
	sp_runtime::traits::{Convert, ConvertBack, SaturatedConversion},
	traits::{
		fungible::{Inspect, Unbalanced},
		tokens::{Fortitude::Polite, Precision::Exact, Preservation::Preserve},
		Get,
	},
	BoundedVec,
};
use frame_system::pallet_prelude::BlockNumberFor;
use nostd::{marker::PhantomData, prelude::*, sync::Arc};
use np_runtime::traits::LateInit;
use serde::{Deserialize, Serialize};
use solana_program_runtime::loaded_programs::ProgramCacheEntry;
use solana_sdk::{
	account::{
		create_account_shared_data_with_fields as create_account, from_account, Account,
		AccountSharedData, InheritableAccountFields, ReadableAccount,
	},
	account_utils::StateMut,
	clock::{Epoch, Slot, UnixTimestamp, INITIAL_RENT_EPOCH},
	epoch_schedule::EpochSchedule,
	feature_set::FeatureSet,
	hash::Hash,
	inner_instruction::InnerInstructions,
	message::SanitizedMessage,
	native_loader,
	nonce::{
		self,
		state::{DurableNonce, State as NonceState, Versions as NonceVersions},
		NONCED_TX_MARKER_IX_INDEX,
	},
	nonce_account,
	pubkey::Pubkey,
	rent_collector::RentCollector,
	sysvar,
	transaction::{Result, SanitizedTransaction, TransactionError},
	transaction_context::{TransactionAccount, TransactionReturnData},
};

#[derive(Serialize, Deserialize)]
pub struct TransactionSimulationResult {
	pub result: Result<()>,
	pub logs: TransactionLogMessages,
	pub post_simulation_accounts: Vec<TransactionAccount>,
	pub units_consumed: u64,
	pub return_data: Option<TransactionReturnData>,
	pub inner_instructions: Option<Vec<InnerInstructions>>,
}

#[derive_where(Default)]
pub struct Bank<T> {
	/// length of a slot in ns
	pub ns_per_slot: u128,
	/// genesis time, used for computed clock
	genesis_creation_time: UnixTimestamp,
	/// Bank slot (i.e. block)
	slot: Slot,
	/// Bank epoch
	epoch: Epoch,
	//collector_id: Pubkey,
	/// latest rent collector, knows the epoch
	rent_collector: RentCollector,
	pub(crate) epoch_schedule: EpochSchedule,
	transaction_processor: LateInit<TransactionProcessor>,
	_marker: PhantomData<T>,
}

impl<T: Config> Bank<T> {
	pub fn new(/* collector_id: Pubkey, */ slot: Slot) -> Self {
		let epoch_schedule = EpochSchedule::without_warmup();
		let epoch = epoch_schedule.get_epoch(slot);
		let genesis_creation_time =
			T::GenesisTimestamp::get().saturated_into::<u64>() as UnixTimestamp;
		let mut bank = Self {
			ns_per_slot: 400_000_000,
			genesis_creation_time,
			slot,
			epoch,
			//collector_id,
			rent_collector: Default::default(),
			epoch_schedule,
			transaction_processor: Default::default(),
			_marker: PhantomData,
		};

		let mut transaction_processor = TransactionProcessor::new(slot, epoch, Default::default());
		// BUILTINS
		transaction_processor.add_builtin(
			&bank,
			solana_system_program::id(),
			"system_program",
			ProgramCacheEntry::new_builtin(
				0,
				"system_program".len(),
				solana_system_program::system_processor::Entrypoint::vm,
			),
		);
		transaction_processor.add_builtin(
			&bank,
			solana_sdk::bpf_loader_deprecated::id(),
			"solana_bpf_loader_deprecated_program",
			ProgramCacheEntry::new_builtin(
				0,
				"solana_bpf_loader_deprecated_program".len(),
				solana_bpf_loader_program::Entrypoint::vm,
			),
		);
		transaction_processor.add_builtin(
			&bank,
			solana_sdk::bpf_loader::id(),
			"solana_bpf_loader_program",
			ProgramCacheEntry::new_builtin(
				0,
				"solana_bpf_loader_program".len(),
				solana_bpf_loader_program::Entrypoint::vm,
			),
		);
		transaction_processor.add_builtin(
			&bank,
			solana_sdk::bpf_loader_upgradeable::id(),
			"solana_bpf_loader_upgradeable_program",
			ProgramCacheEntry::new_builtin(
				0,
				"solana_bpf_loader_upgradeable_program".len(),
				solana_bpf_loader_program::Entrypoint::vm,
			),
		);
		transaction_processor.add_builtin(
			&bank,
			solana_sdk::compute_budget::id(),
			"compute_budget_program",
			ProgramCacheEntry::new_builtin(
				0,
				"compute_budget_program".len(),
				solana_compute_budget_program::Entrypoint::vm,
			),
		);
		transaction_processor.add_builtin(
			&bank,
			solana_sdk::address_lookup_table::program::id(),
			"address_lookup_table_program",
			ProgramCacheEntry::new_builtin(
				0,
				"address_lookup_table_program".len(),
				solana_address_lookup_table_program::processor::Entrypoint::vm,
			),
		);
		// TODO: verify_precompiles.
		// PRECOMPILES
		// self.add_precompile(&solana_sdk::secp256k1_program::id());
		// self.add_precompile(&solana_sdk::ed25519_program::id());

		bank.update_clock(None);
		bank.update_rent();
		bank.update_epoch_schedule();
		//bank.update_recent_blockhashes();
		//bank.update_last_restart_slot();
		transaction_processor.fill_missing_sysvar_cache_entries(&bank);

		bank.transaction_processor.init(transaction_processor);
		bank
	}

	pub fn epoch(&self) -> Epoch {
		self.epoch
	}

	pub fn epoch_schedule(&self) -> &EpochSchedule {
		&self.epoch_schedule
	}

	pub fn load_execute_and_commit_sanitized_transaction(
		&self,
		sanitized_tx: SanitizedTransaction,
	) -> Result<()> {
		let check_result = self.check_transaction(&sanitized_tx, T::BlockhashQueueMaxAge::get());

		let blockhash = T::HashConversion::convert_back(<frame_system::Pallet<T>>::parent_hash());
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

		let mut sanitized_output =
			self.transaction_processor.load_and_execute_sanitized_transaction(
				self,
				&sanitized_tx,
				check_result,
				&processing_environment,
				&processing_config,
			);

		self.commit_transaction(
			&sanitized_tx,
			&mut sanitized_output.loaded_transaction,
			sanitized_output.execution_result.clone(),
			blockhash,
			Default::default(),
		)
	}

	pub fn commit_transaction(
		&self,
		tx: &SanitizedTransaction,
		loaded_tx: &mut TransactionLoadResult,
		execution_result: TransactionExecutionResult,
		last_blockhash: Hash,
		lamports_per_signature: u64,
	) -> Result<()> {
		let durable_nonce = DurableNonce::from_blockhash(&last_blockhash);

		let mut accounts = vec![];
		let mut transactions = vec![];
		if let Ok(loaded_transaction) = loaded_tx {
			let execution_status = match &execution_result {
				TransactionExecutionResult::Executed { details, .. } => &details.status,
				// TODO: error handling.
				TransactionExecutionResult::NotExecuted(e) => return Err(e.clone()),
			};

			// Accounts that are invoked and also not passed as an instruction
			// account to a program don't need to be stored because it's assumed
			// to be impossible for a committable transaction to modify an
			// invoked account if said account isn't passed to some program.
			//
			// Note that this assumption might not hold in the future after
			// SIMD-0082 is implemented because we may decide to commit
			// transactions that incorrectly attempt to invoke a fee payer or
			// durable nonce account. If that happens, we would NOT want to use
			// this filter because we always NEED to store those accounts even
			// if they aren't passed to any programs (because they are mutated
			// outside of the VM).
			let is_storable_account = |message: &SanitizedMessage, key_index: usize| -> bool {
				!message.is_invoked(key_index) || message.is_instruction_account(key_index)
			};

			let message = tx.message();
			let rollback_accounts = &loaded_transaction.rollback_accounts;
			let maybe_nonce_address = rollback_accounts.nonce().map(|account| account.address());

			for (i, (address, account)) in (0..message.account_keys().len())
				.zip(loaded_transaction.accounts.iter_mut())
				.filter(|(i, _)| is_storable_account(message, *i))
			{
				if message.is_writable(i) {
					let should_collect_account = match execution_status {
						Ok(()) => true,
						Err(_) => {
							let is_fee_payer = i == 0;
							let is_nonce_account = Some(&*address) == maybe_nonce_address;
							post_process_failed_tx(
								account,
								is_fee_payer,
								is_nonce_account,
								rollback_accounts,
								&durable_nonce,
								lamports_per_signature,
							);

							is_fee_payer || is_nonce_account
						},
					};

					if should_collect_account {
						// Add to the accounts to store
						accounts.push((&*address, &*account));
						transactions.push(Some(tx));
					}
				}
			}
		}

		// Start storing the accounts.
		if accounts.is_empty() {
			return Ok(());
		}

		// TODO: check has_space_available.
		// TODO: check imbalance.

		for (address, account) in accounts.iter() {
			let pubkey = T::AccountIdConversion::convert(**address);
			let lamports =
				<Lamports<T>>::new(T::Currency::reducible_balance(&pubkey, Preserve, Polite));
			match account.lamports().cmp(&lamports.get()) {
				core::cmp::Ordering::Greater => {
					let amount = <Lamports<T>>::from(account.lamports() - lamports.get());
					// TODO: error handling.
					T::Currency::increase_balance(&pubkey, amount.into_inner(), Exact);
				},
				core::cmp::Ordering::Less => {
					let amount = <Lamports<T>>::from(lamports.get() - account.lamports());
					// TODO: error handling.
					T::Currency::decrease_balance(
						&pubkey,
						amount.into_inner(),
						Exact,
						Preserve,
						Polite,
					);
				},
				_ => {},
			}

			self.store_account(&pubkey, account);
		}

		Ok(())
	}

	fn load_message_nonce_account(
		&self,
		message: &SanitizedMessage,
	) -> Option<(NoncePartial, nonce::state::Data)> {
		let nonce_address = message.get_durable_nonce()?;
		let nonce_account = self.get_account_shared_data(nonce_address)?;
		let nonce_data =
			nonce_account::verify_nonce_account(&nonce_account, message.recent_blockhash())?;

		let nonce_is_authorized = message
			.get_ix_signers(NONCED_TX_MARKER_IX_INDEX as usize)
			.any(|signer| signer == &nonce_data.authority);
		if !nonce_is_authorized {
			return None;
		}

		Some((NoncePartial::new(*nonce_address, nonce_account), nonce_data))
	}

	fn check_and_load_message_nonce_account(
		&self,
		message: &SanitizedMessage,
		next_durable_nonce: &DurableNonce,
	) -> Option<(NoncePartial, nonce::state::Data)> {
		let nonce_is_advanceable = message.recent_blockhash() != next_durable_nonce.as_hash();
		if nonce_is_advanceable {
			self.load_message_nonce_account(message)
		} else {
			None
		}
	}

	pub fn check_transaction(
		&self,
		tx: &SanitizedTransaction,
		//lock_results: &[Result<()>],
		max_age: BlockNumberFor<T>,
		//error_counters: &mut TransactionErrorMetrics,
	) -> TransactionCheckResult {
		let parent_hash = <frame_system::Pallet<T>>::parent_hash();
		let last_blockhash = T::HashConversion::convert_back(parent_hash);
		let next_durable_nonce = DurableNonce::from_blockhash(&last_blockhash);

		let recent_blockhash = T::HashConversion::convert(*tx.message().recent_blockhash());
		if let Some(hash_info) = <Pallet<T>>::get_hash_info_if_valid(&recent_blockhash, max_age) {
			Ok(CheckedTransactionDetails {
				nonce: None,
				lamports_per_signature: hash_info.lamports_per_signature(),
			})
		} else if let Some((nonce, nonce_data)) =
			self.check_and_load_message_nonce_account(tx.message(), &next_durable_nonce)
		{
			Ok(CheckedTransactionDetails {
				nonce: Some(nonce),
				lamports_per_signature: nonce_data.get_lamports_per_signature(),
			})
		} else {
			Err(TransactionError::BlockhashNotFound)
		}

		// TODO: check status cache?
	}

	fn inherit_specially_retained_account_fields(
		&self,
		old_account: &Option<AccountSharedData>,
	) -> InheritableAccountFields {
		const RENT_UNADJUSTED_INITIAL_BALANCE: u64 = 1;

		(
			old_account
				.as_ref()
				.map(|a| a.lamports())
				.unwrap_or(RENT_UNADJUSTED_INITIAL_BALANCE),
			old_account.as_ref().map(|a| a.rent_epoch()).unwrap_or(INITIAL_RENT_EPOCH),
		)
	}

	/// computed unix_timestamp at this slot height
	pub fn unix_timestamp_from_genesis(&self) -> i64 {
		self.genesis_creation_time.saturating_add(
			(self.slot as u128)
				.saturating_mul(self.ns_per_slot)
				.saturating_div(1_000_000_000) as i64,
		)
	}

	fn update_sysvar_account<F>(&self, pubkey: &Pubkey, updater: F)
	where
		F: Fn(&Option<AccountSharedData>) -> AccountSharedData,
	{
		let old_account = self.get_account_shared_data(pubkey);
		let new_account = updater(&old_account);

		// When new sysvar comes into existence (with RENT_UNADJUSTED_INITIAL_BALANCE lamports),
		// this code ensures that the sysvar's balance is adjusted to be rent-exempt.
		//
		// More generally, this code always re-calculates for possible sysvar data size change,
		// although there is no such sysvars currently.
		//self.adjust_sysvar_balance_for_rent(&mut new_account);
		//self.store_account_and_update_capitalization(pubkey, &new_account);
		let pubkey = T::AccountIdConversion::convert(*pubkey);
		self.store_account(&pubkey, &new_account);
	}

	pub fn clock(&self) -> sysvar::clock::Clock {
		from_account(&self.get_account_shared_data(&sysvar::clock::id()).unwrap_or_default())
			.unwrap_or_default()
	}

	fn update_clock(&self, parent_epoch: Option<Epoch>) {
		let mut unix_timestamp = self.clock().unix_timestamp;
		let mut epoch_start_timestamp =
		// On epoch boundaries, update epoch_start_timestamp
		if parent_epoch.is_some() && parent_epoch.unwrap() != self.epoch() {
			unix_timestamp
		} else {
			self.clock().epoch_start_timestamp
		};
		if self.slot == 0 {
			unix_timestamp = self.unix_timestamp_from_genesis();
			epoch_start_timestamp = self.unix_timestamp_from_genesis();
		}
		let clock = sysvar::clock::Clock {
			slot: self.slot,
			epoch_start_timestamp,
			epoch: self.epoch_schedule().get_epoch(self.slot),
			leader_schedule_epoch: self.epoch_schedule().get_leader_schedule_epoch(self.slot),
			unix_timestamp,
		};
		self.update_sysvar_account(&sysvar::clock::id(), |account| {
			create_account(&clock, self.inherit_specially_retained_account_fields(account))
		});
	}

	fn update_rent(&self) {
		self.update_sysvar_account(&sysvar::rent::id(), |account| {
			create_account(
				&self.rent_collector.rent,
				self.inherit_specially_retained_account_fields(account),
			)
		});
	}

	fn update_epoch_schedule(&self) {
		self.update_sysvar_account(&sysvar::epoch_schedule::id(), |account| {
			create_account(
				self.epoch_schedule(),
				self.inherit_specially_retained_account_fields(account),
			)
		});
	}

	fn store_account(&self, pubkey: &T::AccountId, account: &AccountSharedData) {
		<AccountMeta<T>>::mutate(pubkey, |meta| {
			*meta = Some(AccountMetadata {
				rent_epoch: account.rent_epoch(),
				owner: *account.owner(),
				executable: account.executable(),
			});
		});
		if account.data().is_empty() {
			<AccountData<T>>::remove(pubkey);
		} else {
			let data = BoundedVec::try_from(account.data().to_vec()).expect("");
			// TODO: error handling.
			<AccountData<T>>::insert(pubkey, data);
		}
	}

	/*
	fn add_precompile(&self, program_id: &Pubkey) {
		// add_precompiled_account
		let program_id = &T::AccountIdConversion::convert(program_id.clone());
		if let Some(account) = <AccountMeta<T>>::get(program_id) {
			if account.executable {
				return;
			} else {
				// TODO: burn_and_purge_account(program_id, account);
			}
		}

		// Add a bogus executable account, which will be loaded and ignored.
		let (lamports, rent_epoch) = self.inherit_specially_retained_account_fields(&None);

		let account = AccountSharedData::from(Account {
			lamports,
			owner,
			data: vec![],
			executable: true,
			rent_epoch,
		});
		self.store_account(program_id, &account);
	}
	*/
}

impl<T: Config> TransactionProcessingCallback for Bank<T> {
	fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
		let account = T::AccountIdConversion::convert(*account);
		let account = <AccountMeta<T>>::get(account)?;
		owners.iter().position(|entry| account.owner == *entry)
	}

	fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
		let pubkey = T::AccountIdConversion::convert(*pubkey);
		let account = <AccountMeta<T>>::get(&pubkey)?;
		let lamports =
			<Lamports<T>>::new(T::Currency::reducible_balance(&pubkey, Preserve, Polite));
		let data = <AccountData<T>>::get(&pubkey).into();
		Some(AccountSharedData::from(Account {
			lamports: lamports.get(),
			data,
			owner: account.owner,
			executable: account.executable,
			rent_epoch: account.rent_epoch,
		}))
	}

	fn add_builtin_account(&self, name: &str, program_id: &Pubkey) {
		let program_id = &T::AccountIdConversion::convert(*program_id);
		let existing_genuine_program = <AccountMeta<T>>::get(program_id).and_then(|account| {
			// it's very unlikely to be squatted at program_id as non-system account because of
			// burden to find victim's pubkey/hash. So, when account.owner is indeed
			// native_loader's, it's safe to assume it's a genuine program.
			if native_loader::check_id(&account.owner) {
				Some(account)
			} else {
				// TODO: burn_and_purge_account(program_id, account);
				None
			}
		});

		// introducing builtin program
		if existing_genuine_program.is_some() {
			// The existing account is sufficient
			return;
		}

		// Add a bogus executable builtin account, which will be loaded and ignored.
		let account = native_loader::create_loadable_account_with_fields(
			name,
			self.inherit_specially_retained_account_fields(&None),
		);
		self.store_account(program_id, &account);
	}
}

fn post_process_failed_tx(
	account: &mut AccountSharedData,
	is_fee_payer: bool,
	is_nonce_account: bool,
	rollback_accounts: &RollbackAccounts,
	&durable_nonce: &DurableNonce,
	lamports_per_signature: u64,
) {
	// For the case of RollbackAccounts::SameNonceAndFeePayer, it's crucial
	// for `is_nonce_account` to be checked earlier than `is_fee_payer`.
	if is_nonce_account {
		if let Some(nonce) = rollback_accounts.nonce() {
			// The transaction failed which would normally drop the account
			// processing changes, since this account is now being included
			// in the accounts written back to the db, roll it back to
			// pre-processing state.
			*account = nonce.account().clone();

			// Advance the stored blockhash to prevent fee theft by someone
			// replaying nonce transactions that have failed with an
			// `InstructionError`.
			//
			// Since we know we are dealing with a valid nonce account,
			// unwrap is safe here
			let nonce_versions = StateMut::<NonceVersions>::state(account).unwrap();
			if let NonceState::Initialized(ref data) = nonce_versions.state() {
				let nonce_state = NonceState::new_initialized(
					&data.authority,
					durable_nonce,
					lamports_per_signature,
				);
				let nonce_versions = NonceVersions::new(nonce_state);
				account.set_state(&nonce_versions).unwrap();
			}
		}
	} else if is_fee_payer {
		*account = rollback_accounts.fee_payer_account().clone();
	}
}
