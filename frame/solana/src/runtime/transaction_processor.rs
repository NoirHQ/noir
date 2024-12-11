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
	runtime::{
		program_loader::load_program_with_pubkey,
		transaction_processing_callback::TransactionProcessingCallback,
	},
	Config,
};
use nostd::{
	cell::RefCell,
	collections::{hash_map::Entry, HashMap},
	prelude::*,
	rc::Rc,
};
use solana_program_runtime::loaded_programs::ProgramCacheForTxBatch;
use solana_sdk::{
	account::PROGRAM_OWNERS, native_loader, pubkey::Pubkey, saturating_add_assign,
	transaction::SanitizedTransaction,
};

pub struct TransactionProcessor<T>(core::marker::PhantomData<T>);

impl<T: Config> TransactionProcessor<T> {
	fn load_and_execute_sanatized_transaction<CB: TransactionProcessingCallback<T>>(
		&self,
		callbacks: &CB,
		sanitized_tx: SanitizedTransaction,
		//check_result: TransactionCheckResult,
		//environment: &TransactionProcessingEnvironment,
		//config: &TransactionProcessingConfig,
	) {
		// validate_fees

		let mut program_accounts_map =
			Self::filter_executable_program_accounts(callbacks, &sanitized_tx, PROGRAM_OWNERS);
		let native_loader = native_loader::id();
		// builtin_programs

		// TODO: check what bools are for
		let program_cache_for_tx_batch = Rc::new(RefCell::new(self.replenish_program_cache(
			callbacks,
			&program_accounts_map,
			false,
			false,
		)));

		// load_accounts

		// execute_loaded_transaction
	}

	fn execute_loaded_transaction() {}

	/// Returns a map from executable program accounts (all accounts owned by any loader)
	/// to their usage counters, for the transactions with a valid blockhash or nonce.
	fn filter_executable_program_accounts<'a, CB: TransactionProcessingCallback<T>>(
		callbacks: &CB,
		tx: &SanitizedTransaction,
		program_owners: &'a [Pubkey],
	) -> HashMap<Pubkey, (&'a Pubkey, u64)> {
		let mut result: HashMap<Pubkey, (&'a Pubkey, u64)> = HashMap::new();
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

	fn replenish_program_cache<CB: TransactionProcessingCallback<T>>(
		&self,
		callback: &CB,
		program_accounts_map: &HashMap<Pubkey, (&Pubkey, u64)>,
		check_program_modification_slot: bool,
		limit_to_load_programs: bool,
	) -> ProgramCacheForTxBatch {
		let mut loaded_programs_for_tx_batch = ProgramCacheForTxBatch::default();

		loaded_programs_for_tx_batch
	}
}
