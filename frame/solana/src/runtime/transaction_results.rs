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

use crate::{runtime::loaded_programs::ProgramCacheEntry, Config};
use nostd::{collections::HashMap, sync::Arc};
use serde::{Deserialize, Serialize};
// Re-exported since these have moved to `solana_sdk`.
#[deprecated(
	since = "1.18.0",
	note = "Please use `solana_sdk::inner_instruction` types instead"
)]
pub use solana_sdk::inner_instruction::{InnerInstruction, InnerInstructionsList};
use solana_sdk::{
	fee::FeeDetails,
	pubkey::Pubkey,
	rent_debits::RentDebits,
	transaction::{self, TransactionError},
	transaction_context::TransactionReturnData,
};

pub struct TransactionResult<T: Config> {
	pub fee_collection_result: transaction::Result<()>,
	pub loaded_accounts_stat: transaction::Result<TransactionLoadedAccountsStats>,
	pub execution_result: TransactionExecutionResult<T>,
	pub rent_debit: RentDebits,
}

#[derive(Debug, Default, Clone)]
pub struct TransactionLoadedAccountsStats {
	pub loaded_accounts_data_size: usize,
	pub loaded_accounts_count: usize,
}

/// Type safe representation of a transaction execution attempt which
/// differentiates between a transaction that was executed (will be
/// committed to the ledger) and a transaction which wasn't executed
/// and will be dropped.
///
/// Note: `Result<TransactionExecutionDetails, TransactionError>` is not
/// used because it's easy to forget that the inner `details.status` field
/// is what should be checked to detect a successful transaction. This
/// enum provides a convenience method `Self::was_executed_successfully` to
/// make such checks hard to do incorrectly.
#[derive(Debug, Clone)]
pub enum TransactionExecutionResult<T: Config> {
	Executed {
		details: TransactionExecutionDetails,
		programs_modified_by_tx: HashMap<Pubkey, Arc<ProgramCacheEntry<T>>>,
	},
	NotExecuted(TransactionError),
}

impl<T: Config> TransactionExecutionResult<T> {
	pub fn was_executed_successfully(&self) -> bool {
		match self {
			Self::Executed { details, .. } => details.status.is_ok(),
			Self::NotExecuted { .. } => false,
		}
	}

	pub fn was_executed(&self) -> bool {
		match self {
			Self::Executed { .. } => true,
			Self::NotExecuted(_) => false,
		}
	}

	pub fn details(&self) -> Option<&TransactionExecutionDetails> {
		match self {
			Self::Executed { details, .. } => Some(details),
			Self::NotExecuted(_) => None,
		}
	}

	pub fn flattened_result(&self) -> transaction::Result<()> {
		match self {
			Self::Executed { details, .. } => details.status.clone(),
			Self::NotExecuted(err) => Err(err.clone()),
		}
	}
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionExecutionDetails {
	pub status: transaction::Result<()>,
	pub log_messages: Option<Vec<String>>,
	pub inner_instructions: Option<InnerInstructionsList>,
	pub fee_details: FeeDetails,
	pub return_data: Option<TransactionReturnData>,
	pub executed_units: u64,
	/// The change in accounts data len for this transaction.
	/// NOTE: This value is valid IFF `status` is `Ok`.
	pub accounts_data_len_delta: i64,
}
