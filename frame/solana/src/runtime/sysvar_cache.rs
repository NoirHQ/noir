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
