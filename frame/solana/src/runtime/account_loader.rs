// This file is part of Noir.

// Copyright (c) Anza Maintainers <maintainers@anza.xyz>
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
	programs::system::{get_system_account_kind, SystemAccountKind},
	runtime::{
		account::{AccountSharedData, ReadableAccount},
		account_overrides::AccountOverrides,
		account_rent_state::RentState,
		loaded_programs::ProgramCacheForTxBatch,
		nonce_info::NoncePartial,
		rent_collector::{CollectedInfo, RentCollector, RENT_EXEMPT_RENT_EPOCH},
		rollback_accounts::RollbackAccounts,
		transaction_context::{IndexOfAccount, TransactionAccount},
		transaction_error_metrics::TransactionErrorMetrics,
		transaction_processing_callback::TransactionProcessingCallback,
	},
	Config,
};
use itertools::Itertools;
use nostd::{collections::HashMap, num::NonZeroUsize, prelude::*};
use solana_compute_budget::compute_budget_processor::{
	process_compute_budget_instructions, ComputeBudgetLimits,
};
use solana_sdk::{
	feature_set::{self, FeatureSet},
	fee::FeeDetails,
	message::SanitizedMessage,
	native_loader,
	nonce::State as NonceState,
	pubkey::Pubkey,
	rent::RentDue,
	rent_debits::RentDebits,
	saturating_add_assign,
	sysvar::{self, instructions::construct_instructions_data},
	transaction::{Result, SanitizedTransaction, TransactionError},
};

// for the load instructions
pub(crate) type TransactionRent = u64;
pub(crate) type TransactionProgramIndices = Vec<Vec<IndexOfAccount>>;
pub type TransactionCheckResult<T> = Result<CheckedTransactionDetails<T>>;
pub type TransactionValidationResult<T> = Result<ValidatedTransactionDetails<T>>;
pub type TransactionLoadResult<T> = Result<LoadedTransaction<T>>;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct CheckedTransactionDetails<T: Config> {
	pub nonce: Option<NoncePartial<T>>,
	pub lamports_per_signature: u64,
}

#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(feature = "dev-context-only-utils", derive(Default))]
pub struct ValidatedTransactionDetails<T: Config> {
	pub rollback_accounts: RollbackAccounts<T>,
	pub compute_budget_limits: ComputeBudgetLimits,
	pub fee_details: FeeDetails,
	pub fee_payer_account: AccountSharedData<T>,
	pub fee_payer_rent_debit: u64,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct LoadedTransaction<T: Config> {
	pub accounts: Vec<TransactionAccount<T>>,
	pub program_indices: TransactionProgramIndices,
	pub fee_details: FeeDetails,
	pub rollback_accounts: RollbackAccounts<T>,
	pub compute_budget_limits: ComputeBudgetLimits,
	pub rent: TransactionRent,
	pub rent_debits: RentDebits,
	pub loaded_accounts_data_size: usize,
}

/// Collect rent from an account if rent is still enabled and regardless of
/// whether rent is enabled, set the rent epoch to u64::MAX if the account is
/// rent exempt.
pub fn collect_rent_from_account<T: Config>(
	feature_set: &FeatureSet,
	rent_collector: &RentCollector,
	address: &Pubkey,
	account: &mut AccountSharedData<T>,
) -> CollectedInfo {
	if !feature_set.is_active(&feature_set::disable_rent_fees_collection::id()) {
		rent_collector.collect_from_existing_account(address, account)
	} else {
		// When rent fee collection is disabled, we won't collect rent for any account. If there
		// are any rent paying accounts, their `rent_epoch` won't change either. However, if the
		// account itself is rent-exempted but its `rent_epoch` is not u64::MAX, we will set its
		// `rent_epoch` to u64::MAX. In such case, the behavior stays the same as before.
		if account.rent_epoch() != RENT_EXEMPT_RENT_EPOCH &&
			rent_collector.get_rent_due(
				account.lamports(),
				account.data().len(),
				account.rent_epoch(),
			) == RentDue::Exempt
		{
			account.set_rent_epoch(RENT_EXEMPT_RENT_EPOCH);
		}

		CollectedInfo::default()
	}
}

/// Check whether the payer_account is capable of paying the fee. The
/// side effect is to subtract the fee amount from the payer_account
/// balance of lamports. If the payer_acount is not able to pay the
/// fee, the error_metrics is incremented, and a specific error is
/// returned.
pub fn validate_fee_payer<T: Config>(
	payer_address: &Pubkey,
	payer_account: &mut AccountSharedData<T>,
	payer_index: IndexOfAccount,
	error_metrics: &mut TransactionErrorMetrics,
	rent_collector: &RentCollector,
	fee: u64,
) -> Result<()> {
	if payer_account.lamports() == 0 {
		error_metrics.account_not_found += 1;
		return Err(TransactionError::AccountNotFound);
	}
	let system_account_kind = get_system_account_kind(payer_account).ok_or_else(|| {
		error_metrics.invalid_account_for_fee += 1;
		TransactionError::InvalidAccountForFee
	})?;
	let min_balance = match system_account_kind {
		SystemAccountKind::System => 0,
		SystemAccountKind::Nonce => {
			// Should we ever allow a fees charge to zero a nonce account's
			// balance. The state MUST be set to uninitialized in that case
			rent_collector.rent.minimum_balance(NonceState::size())
		},
	};

	payer_account
		.lamports()
		.checked_sub(min_balance)
		.and_then(|v| v.checked_sub(fee))
		.ok_or_else(|| {
			error_metrics.insufficient_funds += 1;
			TransactionError::InsufficientFundsForFee
		})?;

	let payer_pre_rent_state = RentState::from_account(payer_account, &rent_collector.rent);
	payer_account
		.checked_sub_lamports(fee)
		.map_err(|_| TransactionError::InsufficientFundsForFee)?;

	let payer_post_rent_state = RentState::from_account(payer_account, &rent_collector.rent);
	RentState::check_rent_state_with_account(
		&payer_pre_rent_state,
		&payer_post_rent_state,
		payer_address,
		payer_account,
		payer_index,
	)
}

/// Collect information about accounts used in txs transactions and
/// return vector of tuples, one for each transaction in the
/// batch. Each tuple contains struct of information about accounts as
/// its first element and an optional transaction nonce info as its
/// second element.
pub(crate) fn load_accounts<T: Config, CB: TransactionProcessingCallback<T>>(
	callbacks: &CB,
	tx: &SanitizedTransaction,
	validation_result: TransactionValidationResult<T>,
	error_metrics: &mut TransactionErrorMetrics,
	account_overrides: Option<&AccountOverrides<T>>,
	feature_set: &FeatureSet,
	rent_collector: &RentCollector,
	program_accounts: &HashMap<Pubkey, (&Pubkey, u64)>,
	loaded_programs: &ProgramCacheForTxBatch<T>,
) -> TransactionLoadResult<T> {
	match validation_result {
		Ok(tx_details) => {
			let message = tx.message();

			// load transactions
			load_transaction_accounts(
				callbacks,
				message,
				tx_details,
				error_metrics,
				account_overrides,
				feature_set,
				rent_collector,
				program_accounts,
				loaded_programs,
			)
		},
		Err(e) => Err(e),
	}
}

fn load_transaction_accounts<T: Config, CB: TransactionProcessingCallback<T>>(
	callbacks: &CB,
	message: &SanitizedMessage,
	tx_details: ValidatedTransactionDetails<T>,
	error_metrics: &mut TransactionErrorMetrics,
	account_overrides: Option<&AccountOverrides<T>>,
	feature_set: &FeatureSet,
	rent_collector: &RentCollector,
	program_accounts: &HashMap<Pubkey, (&Pubkey, u64)>,
	loaded_programs: &ProgramCacheForTxBatch<T>,
) -> TransactionLoadResult<T> {
	let mut tx_rent: TransactionRent = 0;
	let account_keys = message.account_keys();
	let mut accounts_found = Vec::with_capacity(account_keys.len());
	let mut rent_debits = RentDebits::default();

	let requested_loaded_accounts_data_size_limit =
		get_requested_loaded_accounts_data_size_limit(message)?;
	let mut accumulated_accounts_data_size: usize = 0;

	let disable_account_loader_special_case =
		feature_set.is_active(&feature_set::disable_account_loader_special_case::id());
	let instruction_accounts = message
		.instructions()
		.iter()
		.flat_map(|instruction| &instruction.accounts)
		.unique()
		.collect::<Vec<&u8>>();

	let mut accounts = account_keys
		.iter()
		.enumerate()
		.map(|(i, key)| {
			let mut account_found = true;
			#[allow(clippy::collapsible_else_if)]
			let account = if solana_sdk::sysvar::instructions::check_id(key) {
				construct_instructions_account(message)
			} else {
				let is_fee_payer = i == 0;
				let instruction_account =
					u8::try_from(i).map(|i| instruction_accounts.contains(&&i)).unwrap_or(false);
				let (account_size, account, rent) = if is_fee_payer {
					(
						tx_details.fee_payer_account.data().len(),
						tx_details.fee_payer_account.clone(),
						tx_details.fee_payer_rent_debit,
					)
				} else if let Some(account_override) =
					account_overrides.and_then(|overrides| overrides.get(key))
				{
					(account_override.data().len(), account_override.clone(), 0)
				} else if let Some(program) = (!disable_account_loader_special_case &&
					!instruction_account &&
					!message.is_writable(i))
				.then_some(())
				.and_then(|_| loaded_programs.find(key))
				{
					// Optimization to skip loading of accounts which are only used as
					// programs in top-level instructions and not passed as instruction accounts.
					account_shared_data_from_program(key, program_accounts)
						.map(|program_account| (program.account_size, program_account, 0))?
				} else {
					callbacks
						.get_account_shared_data(key)
						.map(|mut account| {
							if message.is_writable(i) {
								let rent_due = collect_rent_from_account(
									feature_set,
									rent_collector,
									key,
									&mut account,
								)
								.rent_amount;

								(account.data().len(), account, rent_due)
							} else {
								(account.data().len(), account, 0)
							}
						})
						.unwrap_or_else(|| {
							account_found = false;
							let mut default_account = AccountSharedData::default();
							// All new accounts must be rent-exempt (enforced in
							// Bank::execute_loaded_transaction). Currently, rent collection
							// sets rent_epoch to u64::MAX, but initializing the account with
							// this field already set would allow us to skip rent collection for
							// these accounts.
							default_account.set_rent_epoch(RENT_EXEMPT_RENT_EPOCH);
							(default_account.data().len(), default_account, 0)
						})
				};
				accumulate_and_check_loaded_account_data_size(
					&mut accumulated_accounts_data_size,
					account_size,
					requested_loaded_accounts_data_size_limit,
					error_metrics,
				)?;

				tx_rent += rent;
				rent_debits.insert(key, rent, account.lamports());

				account
			};

			accounts_found.push(account_found);
			Ok((*key, account))
		})
		.collect::<Result<Vec<_>>>()?;

	let builtins_start_index = accounts.len();
	let program_indices = message
		.instructions()
		.iter()
		.map(|instruction| {
			let mut account_indices = Vec::with_capacity(2);
			let mut program_index = instruction.program_id_index as usize;
			// This command may never return error, because the transaction is sanitized
			let (program_id, program_account) =
				accounts.get(program_index).ok_or(TransactionError::ProgramAccountNotFound)?;
			if native_loader::check_id(program_id) {
				return Ok(account_indices);
			}

			let account_found = accounts_found.get(program_index).unwrap_or(&true);
			if !account_found {
				error_metrics.account_not_found += 1;
				return Err(TransactionError::ProgramAccountNotFound);
			}

			if !program_account.executable() {
				error_metrics.invalid_program_for_execution += 1;
				return Err(TransactionError::InvalidProgramForExecution);
			}
			account_indices.insert(0, program_index as IndexOfAccount);
			let owner_id = program_account.owner();
			if native_loader::check_id(owner_id) {
				return Ok(account_indices);
			}
			program_index = if let Some(owner_index) = accounts
				.get(builtins_start_index..)
				.ok_or(TransactionError::ProgramAccountNotFound)?
				.iter()
				.position(|(key, _)| key == owner_id)
			{
				builtins_start_index.saturating_add(owner_index)
			} else {
				let owner_index = accounts.len();
				if let Some(owner_account) = callbacks.get_account_shared_data(owner_id) {
					if !native_loader::check_id(owner_account.owner()) ||
						!owner_account.executable()
					{
						error_metrics.invalid_program_for_execution += 1;
						return Err(TransactionError::InvalidProgramForExecution);
					}
					accumulate_and_check_loaded_account_data_size(
						&mut accumulated_accounts_data_size,
						owner_account.data().len(),
						requested_loaded_accounts_data_size_limit,
						error_metrics,
					)?;
					accounts.push((*owner_id, owner_account));
				} else {
					error_metrics.account_not_found += 1;
					return Err(TransactionError::ProgramAccountNotFound);
				}
				owner_index
			};
			account_indices.insert(0, program_index as IndexOfAccount);
			Ok(account_indices)
		})
		.collect::<Result<Vec<Vec<IndexOfAccount>>>>()?;

	Ok(LoadedTransaction {
		accounts,
		program_indices,
		fee_details: tx_details.fee_details,
		rollback_accounts: tx_details.rollback_accounts,
		compute_budget_limits: tx_details.compute_budget_limits,
		rent: tx_rent,
		rent_debits,
		loaded_accounts_data_size: accumulated_accounts_data_size,
	})
}

/// Total accounts data a transaction can load is limited to
///   if `set_tx_loaded_accounts_data_size` instruction is not activated or not used, then
///     default value of 64MiB to not break anyone in Mainnet-beta today
///   else
///     user requested loaded accounts size.
///     Note, requesting zero bytes will result transaction error
fn get_requested_loaded_accounts_data_size_limit(
	sanitized_message: &SanitizedMessage,
) -> Result<Option<NonZeroUsize>> {
	let compute_budget_limits =
		process_compute_budget_instructions(sanitized_message.program_instructions_iter())
			.unwrap_or_default();
	// sanitize against setting size limit to zero
	NonZeroUsize::new(
		usize::try_from(compute_budget_limits.loaded_accounts_bytes).unwrap_or_default(),
	)
	.map_or(Err(TransactionError::InvalidLoadedAccountsDataSizeLimit), |v| Ok(Some(v)))
}

fn account_shared_data_from_program<T: Config>(
	key: &Pubkey,
	program_accounts: &HashMap<Pubkey, (&Pubkey, u64)>,
) -> Result<AccountSharedData<T>> {
	// It's an executable program account. The program is already loaded in the cache.
	// So the account data is not needed. Return a dummy AccountSharedData with meta
	// information.
	let mut program_account = AccountSharedData::default();
	let (program_owner, _count) =
		program_accounts.get(key).ok_or(TransactionError::AccountNotFound)?;
	program_account.set_owner(**program_owner);
	program_account.set_executable(true);
	Ok(program_account)
}

/// Accumulate loaded account data size into `accumulated_accounts_data_size`.
/// Returns TransactionErr::MaxLoadedAccountsDataSizeExceeded if
/// `requested_loaded_accounts_data_size_limit` is specified and
/// `accumulated_accounts_data_size` exceeds it.
fn accumulate_and_check_loaded_account_data_size(
	accumulated_loaded_accounts_data_size: &mut usize,
	account_data_size: usize,
	requested_loaded_accounts_data_size_limit: Option<NonZeroUsize>,
	error_metrics: &mut TransactionErrorMetrics,
) -> Result<()> {
	if let Some(requested_loaded_accounts_data_size) = requested_loaded_accounts_data_size_limit {
		saturating_add_assign!(*accumulated_loaded_accounts_data_size, account_data_size);
		if *accumulated_loaded_accounts_data_size > requested_loaded_accounts_data_size.get() {
			error_metrics.max_loaded_accounts_data_size_exceeded += 1;
			Err(TransactionError::MaxLoadedAccountsDataSizeExceeded)
		} else {
			Ok(())
		}
	} else {
		Ok(())
	}
}

fn construct_instructions_account<T: Config>(message: &SanitizedMessage) -> AccountSharedData<T> {
	AccountSharedData {
		data: construct_instructions_data(&message.decompile_instructions()).into(),
		owner: sysvar::id(),
		..Default::default()
	}
}
