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

use crate::svm::{
	account_loader::{
		collect_rent_from_account, load_accounts, validate_fee_payer, CheckedTransactionDetails,
		LoadedTransaction, TransactionCheckResult, TransactionLoadResult,
		TransactionValidationResult, ValidatedTransactionDetails,
	},
	account_overrides::AccountOverrides,
	message_processor::MessageProcessor,
	program_loader::load_program_with_pubkey,
	rollback_accounts::RollbackAccounts,
	transaction_account_state_info::TransactionAccountStateInfo,
	transaction_error_metrics::TransactionErrorMetrics,
	transaction_processing_callback::TransactionProcessingCallback,
	transaction_results::{TransactionExecutionDetails, TransactionExecutionResult},
};
use nostd::{
	borrow::Borrow,
	cell::RefCell,
	collections::{btree_map::Entry, BTreeMap, BTreeSet},
	prelude::*,
	rc::Rc,
	sync::Arc,
};
use solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1;
use solana_compute_budget::{
	compute_budget::ComputeBudget, compute_budget_processor::process_compute_budget_instructions,
};
use solana_loader_v4_program::create_program_runtime_environment_v2;
use solana_program_runtime::{
	dummy::VoteAccountsHashMap,
	invoke_context::{EnvironmentConfig, InvokeContext},
	loaded_programs::{ProgramCacheEntry, ProgramCacheForTxBatch, ProgramRuntimeEnvironments},
	log_collector::LogCollector,
	sysvar_cache::SysvarCache,
	timings::ExecuteTimings,
};
use solana_sdk::{
	account::{AccountSharedData, ReadableAccount, PROGRAM_OWNERS},
	clock::{Epoch, Slot},
	feature_set::{self, FeatureSet},
	fee::{FeeBudgetLimits, FeeStructure},
	hash::Hash,
	inner_instruction::{InnerInstruction, InnerInstructionsList},
	instruction::{CompiledInstruction, TRANSACTION_LEVEL_STACK_HEIGHT},
	message::SanitizedMessage,
	native_loader,
	pubkey::Pubkey,
	rent_collector::RentCollector,
	saturating_add_assign,
	transaction::{SanitizedTransaction, TransactionError},
	transaction_context::{ExecutionRecord, TransactionContext},
};

/// A list of log messages emitted during a transaction
pub type TransactionLogMessages = Vec<String>;

/// The output of the transaction batch processor's
/// `load_and_execute_sanitized_transactions` method.
pub struct LoadAndExecuteSanitizedTransactionOutput {
	/// Error metrics for transactions that were processed.
	pub error_metrics: TransactionErrorMetrics,
	/// Timings for transaction batch execution.
	pub execute_timings: ExecuteTimings,
	// Vector of results indicating whether a transaction was executed or could not
	// be executed. Note executed transactions can still have failed!
	pub execution_result: TransactionExecutionResult,
	// Vector of loaded transactions from transactions that were processed.
	pub loaded_transaction: TransactionLoadResult,
}

/// Configuration of the recording capabilities for transaction execution
#[derive(Copy, Clone, Default)]
pub struct ExecutionRecordingConfig {
	pub enable_cpi_recording: bool,
	pub enable_log_recording: bool,
	pub enable_return_data_recording: bool,
}

impl ExecutionRecordingConfig {
	pub fn new_single_setting(option: bool) -> Self {
		ExecutionRecordingConfig {
			enable_return_data_recording: option,
			enable_log_recording: option,
			enable_cpi_recording: option,
		}
	}
}
/// Configurations for processing transactions.
#[derive(Default)]
pub struct TransactionProcessingConfig<'a> {
	/// Encapsulates overridden accounts, typically used for transaction
	/// simulation.
	pub account_overrides: Option<&'a AccountOverrides>,
	/// Whether or not to check a program's modification slot when replenishing
	/// a program cache instance.
	pub check_program_modification_slot: bool,
	/// The compute budget to use for transaction execution.
	pub compute_budget: Option<ComputeBudget>,
	/// The maximum number of bytes that log messages can consume.
	pub log_messages_bytes_limit: Option<usize>,
	/// Whether to limit the number of programs loaded for the transaction
	/// batch.
	pub limit_to_load_programs: bool,
	/// Recording capabilities for transaction execution.
	pub recording_config: ExecutionRecordingConfig,
	/// The max number of accounts that a transaction may lock.
	pub transaction_account_lock_limit: Option<usize>,
}

/// Runtime environment for transaction batch processing.
#[derive(Default)]
pub struct TransactionProcessingEnvironment<'a> {
	/// The blockhash to use for the transaction batch.
	pub blockhash: Hash,
	/// The total stake for the current epoch.
	pub epoch_total_stake: Option<u64>,
	/// The vote accounts for the current epoch.
	pub epoch_vote_accounts: Option<&'a VoteAccountsHashMap>,
	/// Runtime feature set to use for the transaction batch.
	pub feature_set: Arc<FeatureSet>,
	/// Fee structure to use for assessing transaction fees.
	pub fee_structure: Option<&'a FeeStructure>,
	/// Lamports per signature to charge per transaction.
	pub lamports_per_signature: u64,
	/// Rent collector to use for the transaction batch.
	pub rent_collector: Option<&'a RentCollector>,
}

#[derive(Default)]
pub struct TransactionProcessor {
	/// Bank slot (i.e. block)
	slot: Slot,
	/// Bank epoch
	epoch: Epoch,
	/// SysvarCache is a collection of system variables that are
	/// accessible from on chain programs. It is passed to SVM from
	/// client code (e.g. Bank) and forwarded to the MessageProcessor.
	pub sysvar_cache: SysvarCache,
	/// Programs required for transaction batch processing
	pub program_cache: BTreeMap<Pubkey, Arc<ProgramCacheEntry>>,
	/// Builtin program ids
	pub builtin_program_ids: BTreeSet<Pubkey>,
}

impl TransactionProcessor {
	pub fn new(slot: Slot, epoch: Epoch, builtin_program_ids: BTreeSet<Pubkey>) -> Self {
		Self {
			slot,
			epoch,
			sysvar_cache: SysvarCache::default(),
			builtin_program_ids,
			..Default::default()
		}
	}

	pub fn new_from(&self, slot: Slot, epoch: Epoch) -> Self {
		Self {
			slot,
			epoch,
			sysvar_cache: SysvarCache::default(),
			builtin_program_ids: self.builtin_program_ids.clone(),
			..Default::default()
		}
	}

	pub fn load_and_execute_sanitized_transaction<CB: TransactionProcessingCallback>(
		&self,
		callbacks: &CB,
		sanitized_tx: &SanitizedTransaction,
		check_result: TransactionCheckResult,
		environment: &TransactionProcessingEnvironment,
		config: &TransactionProcessingConfig,
	) -> LoadAndExecuteSanitizedTransactionOutput {
		// Initialize metrics.
		let mut error_metrics = TransactionErrorMetrics::default();
		let mut execute_timings = ExecuteTimings::default();

		let validation_result = self.validate_fee(
			callbacks,
			config.account_overrides,
			sanitized_tx,
			check_result,
			&environment.feature_set,
			environment.fee_structure.unwrap_or(&FeeStructure::default()),
			environment.rent_collector.unwrap_or(&RentCollector::default()),
			&mut error_metrics,
		);

		let mut program_accounts_map =
			Self::filter_executable_program_accounts(callbacks, sanitized_tx, PROGRAM_OWNERS);
		let native_loader = native_loader::id();
		for builtin_program in self.builtin_program_ids.iter() {
			program_accounts_map.insert(*builtin_program, (&native_loader, 0));
		}

		// TODO: check what bools are for
		let program_cache_for_tx_batch = RefCell::new(self.replenish_program_cache(
			callbacks,
			&program_accounts_map,
			false,
			false,
		));

		let mut loaded_transaction = load_accounts(
			callbacks,
			sanitized_tx,
			validation_result,
			&mut error_metrics,
			config.account_overrides,
			&environment.feature_set,
			environment.rent_collector.unwrap_or(&RentCollector::default()),
			&program_accounts_map,
			&program_cache_for_tx_batch.borrow(),
		);

		let execution_result = match loaded_transaction {
			Err(ref e) => TransactionExecutionResult::NotExecuted(e.clone()),
			Ok(ref mut loaded_transaction) => {
				let result = self.execute_loaded_transaction(
					sanitized_tx,
					loaded_transaction,
					&mut execute_timings,
					&mut error_metrics,
					&mut program_cache_for_tx_batch.borrow_mut(),
					environment,
					config,
				);

				if let TransactionExecutionResult::Executed { details, programs_modified_by_tx } =
					&result
				{
					// Update batch specific cache of the loaded programs with the modifications
					// made by the transaction, if it executed successfully.
					if details.status.is_ok() {
						program_cache_for_tx_batch.borrow_mut().merge(programs_modified_by_tx);
					}
				}

				result
			},
		};

		LoadAndExecuteSanitizedTransactionOutput {
			error_metrics,
			execute_timings,
			execution_result,
			loaded_transaction,
		}
	}

	fn validate_fee<CB: TransactionProcessingCallback>(
		&self,
		callbacks: &CB,
		account_overrides: Option<&AccountOverrides>,
		sanitized_tx: impl Borrow<SanitizedTransaction>,
		check_result: TransactionCheckResult,
		feature_set: &FeatureSet,
		fee_structure: &FeeStructure,
		rent_collector: &RentCollector,
		error_counters: &mut TransactionErrorMetrics,
	) -> TransactionValidationResult {
		check_result.and_then(|checked_details| {
			let message = sanitized_tx.borrow().message();
			self.validate_transaction_fee_payer(
				callbacks,
				account_overrides,
				message,
				checked_details,
				feature_set,
				fee_structure,
				rent_collector,
				error_counters,
			)
		})
	}

	// Loads transaction fee payer, collects rent if necessary, then calculates
	// transaction fees, and deducts them from the fee payer balance. If the
	// account is not found or has insufficient funds, an error is returned.
	fn validate_transaction_fee_payer<CB: TransactionProcessingCallback>(
		&self,
		callbacks: &CB,
		account_overrides: Option<&AccountOverrides>,
		message: &SanitizedMessage,
		checked_details: CheckedTransactionDetails,
		feature_set: &FeatureSet,
		fee_structure: &FeeStructure,
		rent_collector: &RentCollector,
		error_counters: &mut TransactionErrorMetrics,
	) -> TransactionValidationResult {
		let compute_budget_limits =
			process_compute_budget_instructions(message.program_instructions_iter())
				.inspect_err(|_| error_counters.invalid_compute_budget += 1)?;

		let fee_payer_address = message.fee_payer();

		let fee_payer_account = account_overrides
			.and_then(|overrides| overrides.get(fee_payer_address).cloned())
			.or_else(|| callbacks.get_account_shared_data(fee_payer_address));

		let Some(mut fee_payer_account) = fee_payer_account else {
			error_counters.account_not_found += 1;
			return Err(TransactionError::AccountNotFound);
		};

		let fee_payer_loaded_rent_epoch = fee_payer_account.rent_epoch();
		let fee_payer_rent_debit = collect_rent_from_account(
			feature_set,
			rent_collector,
			fee_payer_address,
			&mut fee_payer_account,
		)
		.rent_amount;

		let CheckedTransactionDetails { nonce, lamports_per_signature } = checked_details;

		let fee_budget_limits = FeeBudgetLimits::from(compute_budget_limits);
		let fee_details = fee_structure.calculate_fee_details(
			message,
			lamports_per_signature,
			&fee_budget_limits,
			feature_set
				.is_active(&feature_set::include_loaded_accounts_data_size_in_fee_calculation::id()),
			feature_set.is_active(&feature_set::remove_rounding_in_fee_calculation::id()),
		);

		let fee_payer_index = 0;
		validate_fee_payer(
			fee_payer_address,
			&mut fee_payer_account,
			fee_payer_index,
			error_counters,
			rent_collector,
			fee_details.total_fee(),
		)?;

		// Capture fee-subtracted fee payer account and original nonce account state
		// to rollback to if transaction execution fails.
		let rollback_accounts = RollbackAccounts::new(
			nonce,
			*fee_payer_address,
			fee_payer_account.clone(),
			fee_payer_rent_debit,
			fee_payer_loaded_rent_epoch,
		);

		Ok(ValidatedTransactionDetails {
			fee_details,
			fee_payer_account,
			fee_payer_rent_debit,
			rollback_accounts,
			compute_budget_limits,
		})
	}

	/// Returns a map from executable program accounts (all accounts owned by any loader)
	/// to their usage counters, for the transactions with a valid blockhash or nonce.
	fn filter_executable_program_accounts<'a, CB: TransactionProcessingCallback>(
		callbacks: &CB,
		tx: &SanitizedTransaction,
		program_owners: &'a [Pubkey],
	) -> BTreeMap<Pubkey, (&'a Pubkey, u64)> {
		let mut result: BTreeMap<Pubkey, (&'a Pubkey, u64)> = BTreeMap::new();
		tx.message().account_keys().iter().for_each(|key| match result.entry(*key) {
			Entry::Occupied(mut entry) => {
				let (_, count) = entry.get_mut();
				saturating_add_assign!(*count, 1);
			},
			Entry::Vacant(entry) => {
				if let Some(index) = callbacks.account_matches_owners(key, program_owners) {
					if let Some(owner) = program_owners.get(index) {
						entry.insert((owner, 1));
					}
				}
			},
		});
		result
	}

	fn replenish_program_cache<CB: TransactionProcessingCallback>(
		&self,
		callback: &CB,
		program_accounts_map: &BTreeMap<Pubkey, (&Pubkey, u64)>,
		_check_program_modification_slot: bool,
		_limit_to_load_programs: bool,
	) -> ProgramCacheForTxBatch {
		let mut loaded_programs_for_tx_batch = ProgramCacheForTxBatch::default();

		// FIXME: program_runtime_environments.
		loaded_programs_for_tx_batch.environments = ProgramRuntimeEnvironments {
			program_runtime_v1: Arc::new(
				create_program_runtime_environment_v1(
					&Default::default(),
					&Default::default(),
					false, /* deployment */
					false, /* debugging_features */
				)
				.unwrap(),
			),
			program_runtime_v2: Arc::new(create_program_runtime_environment_v2(
				&Default::default(),
				false, /* debugging_features */
			)),
		};

		// FIXME: load builtins.
		for cached_program in self.program_cache.iter() {
			loaded_programs_for_tx_batch.replenish(*cached_program.0, cached_program.1.clone());
		}

		program_accounts_map
			.iter()
			.filter(|(key, _)| !self.program_cache.contains_key(key))
			.for_each(|(key, _)| {
				let program = load_program_with_pubkey(
					callback,
					// FIXME: program_runtime_environments.
					&loaded_programs_for_tx_batch.environments,
					key,
					self.slot,
					false,
				)
				.expect("called load_program_with_pubkey() with nonexistent account");

				loaded_programs_for_tx_batch.replenish(*key, program);
			});

		loaded_programs_for_tx_batch
	}

	/// Execute a transaction using the provided loaded accounts and update
	/// the executors cache if the transaction was successful.
	#[allow(clippy::too_many_arguments)]
	fn execute_loaded_transaction(
		&self,
		tx: &SanitizedTransaction,
		loaded_transaction: &mut LoadedTransaction,
		execute_timings: &mut ExecuteTimings,
		error_metrics: &mut TransactionErrorMetrics,
		program_cache_for_tx_batch: &mut ProgramCacheForTxBatch,
		environment: &TransactionProcessingEnvironment,
		config: &TransactionProcessingConfig,
	) -> TransactionExecutionResult {
		let transaction_accounts = core::mem::take(&mut loaded_transaction.accounts);

		fn transaction_accounts_lamports_sum(
			accounts: &[(Pubkey, AccountSharedData)],
			message: &SanitizedMessage,
		) -> Option<u128> {
			let mut lamports_sum = 0u128;
			for i in 0..message.account_keys().len() {
				let (_, account) = accounts.get(i)?;
				lamports_sum = lamports_sum.checked_add(u128::from(account.lamports()))?;
			}
			Some(lamports_sum)
		}

		let rent = environment
			.rent_collector
			.map(|rent_collector| rent_collector.rent.clone())
			.unwrap_or_default();

		let lamports_before_tx =
			transaction_accounts_lamports_sum(&transaction_accounts, tx.message()).unwrap_or(0);

		let compute_budget = config
			.compute_budget
			.unwrap_or_else(|| ComputeBudget::from(loaded_transaction.compute_budget_limits));

		let mut transaction_context = TransactionContext::new(
			transaction_accounts,
			rent.clone(),
			compute_budget.max_instruction_stack_depth,
			compute_budget.max_instruction_trace_length,
		);
		#[cfg(debug_assertions)]
		transaction_context.set_signature(tx.signature());

		let pre_account_state_info =
			TransactionAccountStateInfo::new(&rent, &transaction_context, tx.message());

		let log_collector = if config.recording_config.enable_log_recording {
			match config.log_messages_bytes_limit {
				None => Some(LogCollector::new_ref()),
				Some(log_messages_bytes_limit) =>
					Some(LogCollector::new_ref_with_limit(Some(log_messages_bytes_limit))),
			}
		} else {
			None
		};

		let blockhash = environment.blockhash;
		let lamports_per_signature = environment.lamports_per_signature;

		let mut executed_units = 0u64;
		//let sysvar_cache = &self.sysvar_cache.read().unwrap();

		let mut invoke_context = InvokeContext::new(
			&mut transaction_context,
			program_cache_for_tx_batch,
			EnvironmentConfig::new(
				blockhash,
				environment.epoch_total_stake,
				environment.epoch_vote_accounts,
				Arc::clone(&environment.feature_set),
				lamports_per_signature,
				&self.sysvar_cache,
			),
			log_collector.clone(),
			compute_budget,
		);

		let process_result = MessageProcessor::process_message(
			tx.message(),
			&loaded_transaction.program_indices,
			&mut invoke_context,
			execute_timings,
			&mut executed_units,
		);

		drop(invoke_context);

		let mut status = process_result
			.and_then(|info| {
				let post_account_state_info =
					TransactionAccountStateInfo::new(&rent, &transaction_context, tx.message());
				TransactionAccountStateInfo::verify_changes(
					&pre_account_state_info,
					&post_account_state_info,
					&transaction_context,
				)
				.map(|_| info)
			})
			.map_err(|err| {
				match err {
					TransactionError::InvalidRentPayingAccount |
					TransactionError::InsufficientFundsForRent { .. } => {
						error_metrics.invalid_rent_paying_account += 1;
					},
					TransactionError::InvalidAccountIndex => {
						error_metrics.invalid_account_index += 1;
					},
					_ => {
						error_metrics.instruction_error += 1;
					},
				}
				err
			});

		let log_messages: Option<TransactionLogMessages> =
			log_collector.and_then(|log_collector| {
				Rc::try_unwrap(log_collector)
					.map(|log_collector| log_collector.into_inner().into_messages())
					.ok()
			});

		let inner_instructions = if config.recording_config.enable_cpi_recording {
			Some(Self::inner_instructions_list_from_instruction_trace(&transaction_context))
		} else {
			None
		};

		let ExecutionRecord {
			accounts,
			return_data,
			touched_account_count,
			accounts_resize_delta: accounts_data_len_delta,
		} = transaction_context.into();

		if status.is_ok() &&
			transaction_accounts_lamports_sum(&accounts, tx.message())
				.filter(|lamports_after_tx| lamports_before_tx == *lamports_after_tx)
				.is_none()
		{
			status = Err(TransactionError::UnbalancedTransaction);
		}
		let status = status.map(|_| ());

		loaded_transaction.accounts = accounts;

		let return_data = if config.recording_config.enable_return_data_recording &&
			!return_data.data.is_empty()
		{
			Some(return_data)
		} else {
			None
		};

		TransactionExecutionResult::Executed {
			details: TransactionExecutionDetails {
				status,
				log_messages,
				inner_instructions,
				fee_details: loaded_transaction.fee_details,
				return_data,
				executed_units,
				accounts_data_len_delta,
			},
			programs_modified_by_tx: program_cache_for_tx_batch.drain_modified_entries(),
		}
	}

	/// Extract the InnerInstructionsList from a TransactionContext
	fn inner_instructions_list_from_instruction_trace(
		transaction_context: &TransactionContext,
	) -> InnerInstructionsList {
		debug_assert!(transaction_context
			.get_instruction_context_at_index_in_trace(0)
			.map(|instruction_context| instruction_context.get_stack_height() ==
				TRANSACTION_LEVEL_STACK_HEIGHT)
			.unwrap_or(true));
		let mut outer_instructions = Vec::new();
		for index_in_trace in 0..transaction_context.get_instruction_trace_length() {
			if let Ok(instruction_context) =
				transaction_context.get_instruction_context_at_index_in_trace(index_in_trace)
			{
				let stack_height = instruction_context.get_stack_height();
				if stack_height == TRANSACTION_LEVEL_STACK_HEIGHT {
					outer_instructions.push(Vec::new());
				} else if let Some(inner_instructions) = outer_instructions.last_mut() {
					let stack_height = u8::try_from(stack_height).unwrap_or(u8::MAX);
					let instruction = CompiledInstruction::new_from_raw_parts(
						instruction_context
							.get_index_of_program_account_in_transaction(
								instruction_context
									.get_number_of_program_accounts()
									.saturating_sub(1),
							)
							.unwrap_or_default() as u8,
						instruction_context.get_instruction_data().to_vec(),
						(0..instruction_context.get_number_of_instruction_accounts())
							.map(|instruction_account_index| {
								instruction_context
									.get_index_of_instruction_account_in_transaction(
										instruction_account_index,
									)
									.unwrap_or_default() as u8
							})
							.collect(),
					);
					inner_instructions.push(InnerInstruction { instruction, stack_height });
				} else {
					debug_assert!(false);
				}
			} else {
				debug_assert!(false);
			}
		}
		outer_instructions
	}

	pub fn add_builtin<CB: TransactionProcessingCallback>(
		&mut self,
		callbacks: &CB,
		program_id: Pubkey,
		name: &str,
		builtin: ProgramCacheEntry,
	) {
		callbacks.add_builtin_account(name, &program_id);
		self.builtin_program_ids.insert(program_id);
		self.program_cache.insert(program_id, Arc::new(builtin));
	}

	pub fn fill_missing_sysvar_cache_entries<CB: TransactionProcessingCallback>(
		&mut self,
		callbacks: &CB,
	) {
		self.sysvar_cache.fill_missing_entries(|pubkey, set_sysvar| {
			if let Some(account) = callbacks.get_account_shared_data(pubkey) {
				set_sysvar(account.data());
			}
		});
	}
}
