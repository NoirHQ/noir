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

/// These methods facilitate a transition from fetching sysvars from keyed
/// accounts to fetching from the sysvar cache without breaking consensus. In
/// order to keep consistent behavior, they continue to enforce the same checks
/// as `solana_sdk::keyed_account::from_keyed_account` despite dynamically
/// loading them instead of deserializing from account data.
pub mod get_sysvar_with_account_check {
	use crate::{
		runtime::{
			invoke_context::InvokeContext,
			transaction_context::{IndexOfAccount, InstructionContext, TransactionContext},
		},
		Config,
	};
	use nostd::sync::Arc;
	use solana_program_runtime::sysvar_cache::*;
	use solana_sdk::{
		instruction::InstructionError,
		rent::Rent,
		sysvar::{recent_blockhashes::RecentBlockhashes, Sysvar},
	};

	fn check_sysvar_account<S: Sysvar, T: Config>(
		transaction_context: &TransactionContext<T>,
		instruction_context: &InstructionContext<T>,
		instruction_account_index: IndexOfAccount,
	) -> Result<(), InstructionError> {
		let index_in_transaction = instruction_context
			.get_index_of_instruction_account_in_transaction(instruction_account_index)?;
		if !S::check_id(transaction_context.get_key_of_account_at_index(index_in_transaction)?) {
			return Err(InstructionError::InvalidArgument);
		}
		Ok(())
	}

	pub fn rent<T: Config>(
		invoke_context: &InvokeContext<T>,
		instruction_context: &InstructionContext<T>,
		instruction_account_index: IndexOfAccount,
	) -> Result<Arc<Rent>, InstructionError> {
		check_sysvar_account::<Rent, _>(
			invoke_context.transaction_context,
			instruction_context,
			instruction_account_index,
		)?;
		invoke_context.get_sysvar_cache().get_rent()
	}

	#[allow(deprecated)]
	pub fn recent_blockhashes<T: Config>(
		invoke_context: &InvokeContext<T>,
		instruction_context: &InstructionContext<T>,
		instruction_account_index: IndexOfAccount,
	) -> Result<Arc<RecentBlockhashes>, InstructionError> {
		check_sysvar_account::<RecentBlockhashes, _>(
			invoke_context.transaction_context,
			instruction_context,
			instruction_account_index,
		)?;
		invoke_context.get_sysvar_cache().get_recent_blockhashes()
	}
}
