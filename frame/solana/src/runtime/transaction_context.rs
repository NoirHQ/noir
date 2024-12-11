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
		account::{AccountSharedData, ReadableAccount},
		Lamports,
	},
	BalanceOf, Config,
};
use nostd::{
	cell::{Ref, RefCell, RefMut},
	collections::HashSet,
	marker::PhantomData,
	mem::MaybeUninit,
	pin::Pin,
	prelude::*,
	rc::Rc,
};
pub use solana_sdk::transaction_context::{
	IndexOfAccount, InstructionAccount, TransactionReturnData,
};
use solana_sdk::{
	entrypoint::MAX_PERMITTED_DATA_INCREASE,
	instruction::InstructionError,
	pubkey::Pubkey,
	rent::Rent,
	signature::Signature,
	system_instruction::{
		MAX_PERMITTED_ACCOUNTS_DATA_ALLOCATIONS_PER_TRANSACTION, MAX_PERMITTED_DATA_LENGTH,
	},
};

/// An account key and the matching account
pub type TransactionAccount<T> = (Pubkey, AccountSharedData<T>);

#[derive(Clone, PartialEq)]
#[derive_where(Debug)]
pub struct TransactionAccounts<T: Config> {
	accounts: Vec<RefCell<AccountSharedData<T>>>,
	touched_flags: RefCell<Box<[bool]>>,
}

impl<T: Config> TransactionAccounts<T> {
	#[cfg(not(target_os = "solana"))]
	fn new(accounts: Vec<RefCell<AccountSharedData<T>>>) -> TransactionAccounts<T> {
		TransactionAccounts {
			touched_flags: RefCell::new(vec![false; accounts.len()].into_boxed_slice()),
			accounts,
		}
	}

	fn len(&self) -> usize {
		self.accounts.len()
	}

	pub fn get(&self, index: IndexOfAccount) -> Option<&RefCell<AccountSharedData<T>>> {
		self.accounts.get(index as usize)
	}

	#[cfg(not(target_os = "solana"))]
	pub fn touch(&self, index: IndexOfAccount) -> Result<(), InstructionError> {
		*self
			.touched_flags
			.borrow_mut()
			.get_mut(index as usize)
			.ok_or(InstructionError::NotEnoughAccountKeys)? = true;
		Ok(())
	}

	#[cfg(not(target_os = "solana"))]
	pub fn touched_count(&self) -> usize {
		self.touched_flags.borrow().iter().fold(0usize, |accumulator, was_touched| {
			accumulator.saturating_add(*was_touched as usize)
		})
	}

	pub fn try_borrow(
		&self,
		index: IndexOfAccount,
	) -> Result<Ref<'_, AccountSharedData<T>>, InstructionError> {
		self.accounts
			.get(index as usize)
			.ok_or(InstructionError::MissingAccount)?
			.try_borrow()
			.map_err(|_| InstructionError::AccountBorrowFailed)
	}

	pub fn try_borrow_mut(
		&self,
		index: IndexOfAccount,
	) -> Result<RefMut<'_, AccountSharedData<T>>, InstructionError> {
		self.accounts
			.get(index as usize)
			.ok_or(InstructionError::MissingAccount)?
			.try_borrow_mut()
			.map_err(|_| InstructionError::AccountBorrowFailed)
	}

	pub fn into_accounts(self) -> Vec<AccountSharedData<T>> {
		self.accounts.into_iter().map(|account| account.into_inner()).collect()
	}
}

/// Loaded transaction shared between runtime and programs.
///
/// This context is valid for the entire duration of a transaction being processed.
#[derive(Clone, PartialEq)]
#[derive_where(Debug)]
pub struct TransactionContext<T: Config> {
	account_keys: Pin<Box<[Pubkey]>>,
	accounts: Rc<TransactionAccounts<T>>,
	instruction_stack_capacity: usize,
	instruction_trace_capacity: usize,
	instruction_stack: Vec<usize>,
	instruction_trace: Vec<InstructionContext<T>>,
	return_data: TransactionReturnData,
	accounts_resize_delta: RefCell<i64>,
	#[cfg(not(target_os = "solana"))]
	rent: Rent,
	/// Useful for debugging to filter by or to look it up on the explorer
	#[cfg(all(not(target_os = "solana"), debug_assertions))]
	signature: Signature,
}

impl<T: Config> TransactionContext<T> {
	/// Constructs a new TransactionContext
	#[cfg(not(target_os = "solana"))]
	pub fn new(
		transaction_accounts: Vec<TransactionAccount<T>>,
		rent: Rent,
		instruction_stack_capacity: usize,
		instruction_trace_capacity: usize,
	) -> Self {
		let (account_keys, accounts): (Vec<_>, Vec<_>) = transaction_accounts
			.into_iter()
			.map(|(key, account)| (key, RefCell::new(account)))
			.unzip();
		Self {
			account_keys: Pin::new(account_keys.into_boxed_slice()),
			accounts: Rc::new(TransactionAccounts::new(accounts)),
			instruction_stack_capacity,
			instruction_trace_capacity,
			instruction_stack: Vec::with_capacity(instruction_stack_capacity),
			instruction_trace: vec![InstructionContext::default()],
			return_data: TransactionReturnData::default(),
			accounts_resize_delta: RefCell::new(0),
			rent,
			#[cfg(all(not(target_os = "solana"), debug_assertions))]
			signature: Signature::default(),
		}
	}

	/// Used in mock_process_instruction
	#[cfg(not(target_os = "solana"))]
	pub fn deconstruct_without_keys(self) -> Result<Vec<AccountSharedData<T>>, InstructionError> {
		if !self.instruction_stack.is_empty() {
			return Err(InstructionError::CallDepth);
		}

		Ok(Rc::try_unwrap(self.accounts)
			.expect("transaction_context.accounts has unexpected outstanding refs")
			.into_accounts())
	}

	#[cfg(not(target_os = "solana"))]
	pub fn accounts(&self) -> &Rc<TransactionAccounts<T>> {
		&self.accounts
	}

	/// Stores the signature of the current transaction
	#[cfg(all(not(target_os = "solana"), debug_assertions))]
	pub fn set_signature(&mut self, signature: &Signature) {
		self.signature = *signature;
	}

	/// Returns the signature of the current transaction
	#[cfg(all(not(target_os = "solana"), debug_assertions))]
	pub fn get_signature(&self) -> &Signature {
		&self.signature
	}

	/// Returns the total number of accounts loaded in this Transaction
	pub fn get_number_of_accounts(&self) -> IndexOfAccount {
		self.accounts.len() as IndexOfAccount
	}

	/// Searches for an account by its key
	pub fn get_key_of_account_at_index(
		&self,
		index_in_transaction: IndexOfAccount,
	) -> Result<&Pubkey, InstructionError> {
		self.account_keys
			.get(index_in_transaction as usize)
			.ok_or(InstructionError::NotEnoughAccountKeys)
	}

	/// Searches for an account by its key
	#[cfg(not(target_os = "solana"))]
	pub fn get_account_at_index(
		&self,
		index_in_transaction: IndexOfAccount,
	) -> Result<&RefCell<AccountSharedData<T>>, InstructionError> {
		self.accounts
			.get(index_in_transaction)
			.ok_or(InstructionError::NotEnoughAccountKeys)
	}

	/// Searches for an account by its key
	pub fn find_index_of_account(&self, pubkey: &Pubkey) -> Option<IndexOfAccount> {
		self.account_keys
			.iter()
			.position(|key| key == pubkey)
			.map(|index| index as IndexOfAccount)
	}

	/// Searches for a program account by its key
	pub fn find_index_of_program_account(&self, pubkey: &Pubkey) -> Option<IndexOfAccount> {
		self.account_keys
			.iter()
			.rposition(|key| key == pubkey)
			.map(|index| index as IndexOfAccount)
	}

	/// Gets the max length of the InstructionContext trace
	pub fn get_instruction_trace_capacity(&self) -> usize {
		self.instruction_trace_capacity
	}

	/// Returns the instruction trace length.
	///
	/// Not counting the last empty InstructionContext which is always pre-reserved for the next
	/// instruction. See also `get_next_instruction_context()`.
	pub fn get_instruction_trace_length(&self) -> usize {
		self.instruction_trace.len().saturating_sub(1)
	}

	/// Gets an InstructionContext by its index in the trace
	pub fn get_instruction_context_at_index_in_trace(
		&self,
		index_in_trace: usize,
	) -> Result<&InstructionContext<T>, InstructionError> {
		self.instruction_trace.get(index_in_trace).ok_or(InstructionError::CallDepth)
	}

	/// Gets an InstructionContext by its nesting level in the stack
	pub fn get_instruction_context_at_nesting_level(
		&self,
		nesting_level: usize,
	) -> Result<&InstructionContext<T>, InstructionError> {
		let index_in_trace =
			*self.instruction_stack.get(nesting_level).ok_or(InstructionError::CallDepth)?;
		let instruction_context = self.get_instruction_context_at_index_in_trace(index_in_trace)?;
		debug_assert_eq!(instruction_context.nesting_level, nesting_level);
		Ok(instruction_context)
	}

	/// Gets the max height of the InstructionContext stack
	pub fn get_instruction_stack_capacity(&self) -> usize {
		self.instruction_stack_capacity
	}

	/// Gets instruction stack height, top-level instructions are height
	/// `solana_sdk::instruction::TRANSACTION_LEVEL_STACK_HEIGHT`
	pub fn get_instruction_context_stack_height(&self) -> usize {
		self.instruction_stack.len()
	}

	/// Returns the current InstructionContext
	pub fn get_current_instruction_context(
		&self,
	) -> Result<&InstructionContext<T>, InstructionError> {
		let level = self
			.get_instruction_context_stack_height()
			.checked_sub(1)
			.ok_or(InstructionError::CallDepth)?;
		self.get_instruction_context_at_nesting_level(level)
	}

	/// Returns the InstructionContext to configure for the next invocation.
	///
	/// The last InstructionContext is always empty and pre-reserved for the next instruction.
	pub fn get_next_instruction_context(
		&mut self,
	) -> Result<&mut InstructionContext<T>, InstructionError> {
		self.instruction_trace.last_mut().ok_or(InstructionError::CallDepth)
	}

	/// Pushes the next InstructionContext
	#[cfg(not(target_os = "solana"))]
	pub fn push(&mut self) -> Result<(), InstructionError> {
		let nesting_level = self.get_instruction_context_stack_height();
		let caller_instruction_context =
			self.instruction_trace.last().ok_or(InstructionError::CallDepth)?;
		let callee_instruction_accounts_lamport_sum =
			self.instruction_accounts_lamport_sum(caller_instruction_context)?;
		if !self.instruction_stack.is_empty() {
			let caller_instruction_context = self.get_current_instruction_context()?;
			let original_caller_instruction_accounts_lamport_sum =
				caller_instruction_context.instruction_accounts_lamport_sum;
			let current_caller_instruction_accounts_lamport_sum =
				self.instruction_accounts_lamport_sum(caller_instruction_context)?;
			if original_caller_instruction_accounts_lamport_sum !=
				current_caller_instruction_accounts_lamport_sum
			{
				return Err(InstructionError::UnbalancedInstruction);
			}
		}
		{
			let instruction_context = self.get_next_instruction_context()?;
			instruction_context.nesting_level = nesting_level;
			instruction_context.instruction_accounts_lamport_sum =
				callee_instruction_accounts_lamport_sum;
		}
		let index_in_trace = self.get_instruction_trace_length();
		if index_in_trace >= self.instruction_trace_capacity {
			return Err(InstructionError::MaxInstructionTraceLengthExceeded);
		}
		self.instruction_trace.push(InstructionContext::default());
		if nesting_level >= self.instruction_stack_capacity {
			return Err(InstructionError::CallDepth);
		}
		self.instruction_stack.push(index_in_trace);
		Ok(())
	}

	/// Pops the current InstructionContext
	#[cfg(not(target_os = "solana"))]
	pub fn pop(&mut self) -> Result<(), InstructionError> {
		if self.instruction_stack.is_empty() {
			return Err(InstructionError::CallDepth);
		}
		// Verify (before we pop) that the total sum of all lamports in this instruction did not
		// change
		let detected_an_unbalanced_instruction =
			self.get_current_instruction_context().and_then(|instruction_context| {
				// Verify all executable accounts have no outstanding refs
				for account_index in instruction_context.program_accounts.iter() {
					self.get_account_at_index(*account_index)?
						.try_borrow_mut()
						.map_err(|_| InstructionError::AccountBorrowOutstanding)?;
				}
				self.instruction_accounts_lamport_sum(instruction_context).map(
					|instruction_accounts_lamport_sum| {
						instruction_context.instruction_accounts_lamport_sum !=
							instruction_accounts_lamport_sum
					},
				)
			});
		// Always pop, even if we `detected_an_unbalanced_instruction`
		self.instruction_stack.pop();
		if detected_an_unbalanced_instruction? {
			Err(InstructionError::UnbalancedInstruction)
		} else {
			Ok(())
		}
	}

	/// Gets the return data of the current InstructionContext or any above
	pub fn get_return_data(&self) -> (&Pubkey, &[u8]) {
		(&self.return_data.program_id, &self.return_data.data)
	}

	/// Set the return data of the current InstructionContext
	pub fn set_return_data(
		&mut self,
		program_id: Pubkey,
		data: Vec<u8>,
	) -> Result<(), InstructionError> {
		self.return_data = TransactionReturnData { program_id, data };
		Ok(())
	}

	/// Calculates the sum of all lamports within an instruction
	#[cfg(not(target_os = "solana"))]
	fn instruction_accounts_lamport_sum(
		&self,
		instruction_context: &InstructionContext<T>,
	) -> Result<u128, InstructionError> {
		let mut instruction_accounts_lamport_sum: u128 = 0;
		for instruction_account_index in 0..instruction_context.get_number_of_instruction_accounts()
		{
			if instruction_context
				.is_instruction_account_duplicate(instruction_account_index)?
				.is_some()
			{
				continue; // Skip duplicate account
			}
			let index_in_transaction = instruction_context
				.get_index_of_instruction_account_in_transaction(instruction_account_index)?;
			instruction_accounts_lamport_sum = (self
				.get_account_at_index(index_in_transaction)?
				.try_borrow()
				.map_err(|_| InstructionError::AccountBorrowOutstanding)?
				.lamports() as u128)
				.checked_add(instruction_accounts_lamport_sum)
				.ok_or(InstructionError::ArithmeticOverflow)?;
		}
		Ok(instruction_accounts_lamport_sum)
	}

	/// Returns the accounts resize delta
	pub fn accounts_resize_delta(&self) -> Result<i64, InstructionError> {
		self.accounts_resize_delta
			.try_borrow()
			.map_err(|_| InstructionError::GenericError)
			.map(|value_ref| *value_ref)
	}
}

/// Loaded instruction shared between runtime and programs.
///
/// This context is valid for the entire duration of a (possibly cross program) instruction being
/// processed.
#[derive(Clone, Eq, PartialEq)]
#[derive_where(Debug)]
pub struct InstructionContext<T: Config> {
	nesting_level: usize,
	instruction_accounts_lamport_sum: u128,
	program_accounts: Vec<IndexOfAccount>,
	instruction_accounts: Vec<InstructionAccount>,
	instruction_data: Vec<u8>,
	_marker: PhantomData<T>,
}

impl<T: Config> InstructionContext<T> {
	/// Used together with TransactionContext::get_next_instruction_context()
	#[cfg(not(target_os = "solana"))]
	pub fn configure(
		&mut self,
		program_accounts: &[IndexOfAccount],
		instruction_accounts: &[InstructionAccount],
		instruction_data: &[u8],
	) {
		self.program_accounts = program_accounts.to_vec();
		self.instruction_accounts = instruction_accounts.to_vec();
		self.instruction_data = instruction_data.to_vec();
	}

	/// How many Instructions were on the stack after this one was pushed
	///
	/// That is the number of nested parent Instructions plus one (itself).
	pub fn get_stack_height(&self) -> usize {
		self.nesting_level.saturating_add(1)
	}

	/// Number of program accounts
	pub fn get_number_of_program_accounts(&self) -> IndexOfAccount {
		self.program_accounts.len() as IndexOfAccount
	}

	/// Number of accounts in this Instruction (without program accounts)
	pub fn get_number_of_instruction_accounts(&self) -> IndexOfAccount {
		self.instruction_accounts.len() as IndexOfAccount
	}

	/// Assert that enough accounts were supplied to this Instruction
	pub fn check_number_of_instruction_accounts(
		&self,
		expected_at_least: IndexOfAccount,
	) -> Result<(), InstructionError> {
		if self.get_number_of_instruction_accounts() < expected_at_least {
			Err(InstructionError::NotEnoughAccountKeys)
		} else {
			Ok(())
		}
	}

	/// Data parameter for the programs `process_instruction` handler
	pub fn get_instruction_data(&self) -> &[u8] {
		&self.instruction_data
	}

	/// Searches for a program account by its key
	pub fn find_index_of_program_account(
		&self,
		transaction_context: &TransactionContext<T>,
		pubkey: &Pubkey,
	) -> Option<IndexOfAccount> {
		self.program_accounts
			.iter()
			.position(|index_in_transaction| {
				transaction_context.account_keys.get(*index_in_transaction as usize) == Some(pubkey)
			})
			.map(|index| index as IndexOfAccount)
	}

	/// Searches for an instruction account by its key
	pub fn find_index_of_instruction_account(
		&self,
		transaction_context: &TransactionContext<T>,
		pubkey: &Pubkey,
	) -> Option<IndexOfAccount> {
		self.instruction_accounts
			.iter()
			.position(|instruction_account| {
				transaction_context
					.account_keys
					.get(instruction_account.index_in_transaction as usize) ==
					Some(pubkey)
			})
			.map(|index| index as IndexOfAccount)
	}

	/// Translates the given instruction wide program_account_index into a transaction wide index
	pub fn get_index_of_program_account_in_transaction(
		&self,
		program_account_index: IndexOfAccount,
	) -> Result<IndexOfAccount, InstructionError> {
		Ok(*self
			.program_accounts
			.get(program_account_index as usize)
			.ok_or(InstructionError::NotEnoughAccountKeys)?)
	}

	/// Translates the given instruction wide instruction_account_index into a transaction wide
	/// index
	pub fn get_index_of_instruction_account_in_transaction(
		&self,
		instruction_account_index: IndexOfAccount,
	) -> Result<IndexOfAccount, InstructionError> {
		Ok(self
			.instruction_accounts
			.get(instruction_account_index as usize)
			.ok_or(InstructionError::NotEnoughAccountKeys)?
			.index_in_transaction as IndexOfAccount)
	}

	/// Returns `Some(instruction_account_index)` if this is a duplicate
	/// and `None` if it is the first account with this key
	pub fn is_instruction_account_duplicate(
		&self,
		instruction_account_index: IndexOfAccount,
	) -> Result<Option<IndexOfAccount>, InstructionError> {
		let index_in_callee = self
			.instruction_accounts
			.get(instruction_account_index as usize)
			.ok_or(InstructionError::NotEnoughAccountKeys)?
			.index_in_callee;
		Ok(if index_in_callee == instruction_account_index { None } else { Some(index_in_callee) })
	}

	/// Gets the key of the last program account of this Instruction
	pub fn get_last_program_key<'a, 'b: 'a>(
		&'a self,
		transaction_context: &'b TransactionContext<T>,
	) -> Result<&'b Pubkey, InstructionError> {
		self.get_index_of_program_account_in_transaction(
			self.get_number_of_program_accounts().saturating_sub(1),
		)
		.and_then(|index_in_transaction| {
			transaction_context.get_key_of_account_at_index(index_in_transaction)
		})
	}

	fn try_borrow_account<'a, 'b: 'a>(
		&'a self,
		transaction_context: &'b TransactionContext<T>,
		index_in_transaction: IndexOfAccount,
		index_in_instruction: IndexOfAccount,
	) -> Result<BorrowedAccount<'a, T>, InstructionError> {
		let account = transaction_context
			.accounts
			.get(index_in_transaction)
			.ok_or(InstructionError::MissingAccount)?
			.try_borrow_mut()
			.map_err(|_| InstructionError::AccountBorrowFailed)?;
		Ok(BorrowedAccount {
			transaction_context,
			instruction_context: self,
			index_in_transaction,
			index_in_instruction,
			account,
		})
	}

	/// Gets the last program account of this Instruction
	pub fn try_borrow_last_program_account<'a, 'b: 'a>(
		&'a self,
		transaction_context: &'b TransactionContext<T>,
	) -> Result<BorrowedAccount<'a, T>, InstructionError> {
		let result = self.try_borrow_program_account(
			transaction_context,
			self.get_number_of_program_accounts().saturating_sub(1),
		);
		debug_assert!(result.is_ok());
		result
	}

	/// Tries to borrow a program account from this Instruction
	pub fn try_borrow_program_account<'a, 'b: 'a>(
		&'a self,
		transaction_context: &'b TransactionContext<T>,
		program_account_index: IndexOfAccount,
	) -> Result<BorrowedAccount<'a, T>, InstructionError> {
		let index_in_transaction =
			self.get_index_of_program_account_in_transaction(program_account_index)?;
		self.try_borrow_account(transaction_context, index_in_transaction, program_account_index)
	}

	/// Gets an instruction account of this Instruction
	pub fn try_borrow_instruction_account<'a, 'b: 'a>(
		&'a self,
		transaction_context: &'b TransactionContext<T>,
		instruction_account_index: IndexOfAccount,
	) -> Result<BorrowedAccount<'a, T>, InstructionError> {
		let index_in_transaction =
			self.get_index_of_instruction_account_in_transaction(instruction_account_index)?;
		self.try_borrow_account(
			transaction_context,
			index_in_transaction,
			self.get_number_of_program_accounts().saturating_add(instruction_account_index),
		)
	}

	/// Returns whether an instruction account is a signer
	pub fn is_instruction_account_signer(
		&self,
		instruction_account_index: IndexOfAccount,
	) -> Result<bool, InstructionError> {
		Ok(self
			.instruction_accounts
			.get(instruction_account_index as usize)
			.ok_or(InstructionError::MissingAccount)?
			.is_signer)
	}

	/// Returns whether an instruction account is writable
	pub fn is_instruction_account_writable(
		&self,
		instruction_account_index: IndexOfAccount,
	) -> Result<bool, InstructionError> {
		Ok(self
			.instruction_accounts
			.get(instruction_account_index as usize)
			.ok_or(InstructionError::MissingAccount)?
			.is_writable)
	}

	/// Calculates the set of all keys of signer instruction accounts in this Instruction
	pub fn get_signers(
		&self,
		transaction_context: &TransactionContext<T>,
	) -> Result<HashSet<Pubkey>, InstructionError> {
		let mut result = HashSet::new();
		for instruction_account in self.instruction_accounts.iter() {
			if instruction_account.is_signer {
				result.insert(
					*transaction_context
						.get_key_of_account_at_index(instruction_account.index_in_transaction)?,
				);
			}
		}
		Ok(result)
	}
}

impl<T: Config> Default for InstructionContext<T> {
	fn default() -> Self {
		Self {
			nesting_level: 0,
			instruction_accounts_lamport_sum: 0,
			program_accounts: Vec::new(),
			instruction_accounts: Vec::new(),
			instruction_data: Vec::new(),
			_marker: PhantomData,
		}
	}
}

/// Shared account borrowed from the TransactionContext and an InstructionContext.
#[derive_where(Debug)]
pub struct BorrowedAccount<'a, T: Config> {
	transaction_context: &'a TransactionContext<T>,
	instruction_context: &'a InstructionContext<T>,
	index_in_transaction: IndexOfAccount,
	index_in_instruction: IndexOfAccount,
	account: RefMut<'a, AccountSharedData<T>>,
}

impl<'a, T: Config> BorrowedAccount<'a, T> {
	/// Returns the transaction context
	pub fn transaction_context(&self) -> &TransactionContext<T> {
		self.transaction_context
	}

	/// Returns the index of this account (transaction wide)
	#[inline]
	pub fn get_index_in_transaction(&self) -> IndexOfAccount {
		self.index_in_transaction
	}

	/// Returns the public key of this account (transaction wide)
	#[inline]
	pub fn get_key(&self) -> &Pubkey {
		self.transaction_context
			.get_key_of_account_at_index(self.index_in_transaction)
			.unwrap()
	}

	/// Returns the owner of this account (transaction wide)
	#[inline]
	pub fn get_owner(&self) -> &Pubkey {
		self.account.owner()
	}

	/// Assignes the owner of this account (transaction wide)
	#[cfg(not(target_os = "solana"))]
	pub fn set_owner(&mut self, pubkey: &[u8]) -> Result<(), InstructionError> {
		// Only the owner can assign a new owner
		if !self.is_owned_by_current_program() {
			return Err(InstructionError::ModifiedProgramId);
		}
		// and only if the account is writable
		if !self.is_writable() {
			return Err(InstructionError::ModifiedProgramId);
		}
		// and only if the account is not executable
		if self.is_executable() {
			return Err(InstructionError::ModifiedProgramId);
		}
		// and only if the data is zero-initialized or empty
		if !is_zeroed(self.get_data()) {
			return Err(InstructionError::ModifiedProgramId);
		}
		// don't touch the account if the owner does not change
		if self.get_owner().to_bytes() == pubkey {
			return Ok(());
		}
		self.touch()?;
		self.account.copy_into_owner_from_slice(pubkey);
		Ok(())
	}

	/*
	/// Returns the number of lamports of this account (transaction wide)
	#[inline]
	pub fn get_lamports(&self) -> u64 {
		self.account.lamports()
	}

	/// Overwrites the number of lamports of this account (transaction wide)
	#[cfg(not(target_os = "solana"))]
	pub fn set_lamports(&mut self, lamports: u64) -> Result<(), InstructionError> {
		// An account not owned by the program cannot have its balance decrease
		if !self.is_owned_by_current_program() && lamports < self.get_lamports() {
			return Err(InstructionError::ExternalAccountLamportSpend);
		}
		// The balance of read-only may not change
		if !self.is_writable() {
			return Err(InstructionError::ReadonlyLamportChange);
		}
		// The balance of executable accounts may not change
		if self.is_executable() {
			return Err(InstructionError::ExecutableLamportChange);
		}
		// don't touch the account if the lamports do not change
		if self.get_lamports() == lamports {
			return Ok(());
		}
		self.touch()?;
		self.account.set_lamports(lamports);
		Ok(())
	}

	/// Adds lamports to this account (transaction wide)
	#[cfg(not(target_os = "solana"))]
	pub fn checked_add_lamports(&mut self, lamports: u64) -> Result<(), InstructionError> {
		self.set_lamports(
			self.get_lamports()
				.checked_add(lamports)
				.ok_or(InstructionError::ArithmeticOverflow)?,
		)
	}

	/// Subtracts lamports from this account (transaction wide)
	#[cfg(not(target_os = "solana"))]
	pub fn checked_sub_lamports(&mut self, lamports: u64) -> Result<(), InstructionError> {
		self.set_lamports(
			self.get_lamports()
				.checked_sub(lamports)
				.ok_or(InstructionError::ArithmeticOverflow)?,
		)
	}
	*/

	/// Returns the number of lamports of this account (transaction wide)
	#[inline]
	pub fn get_lamports(&self) -> u64 {
		self.account.lamports()
	}

	/// Returns the number of lamports of this account (transaction wide)
	#[inline]
	pub fn lamports(&self) -> Lamports<T> {
		self.account.get_lamports()
	}

	/// Overwrites the number of lamports of this account (transaction wide)
	#[cfg(not(target_os = "solana"))]
	pub fn set_lamports(&mut self, lamports: Lamports<T>) -> Result<(), InstructionError> {
		// An account not owned by the program cannot have its balance decrease
		if !self.is_owned_by_current_program() && lamports < self.lamports() {
			return Err(InstructionError::ExternalAccountLamportSpend);
		}
		// The balance of read-only may not change
		if !self.is_writable() {
			return Err(InstructionError::ReadonlyLamportChange);
		}
		// The balance of executable accounts may not change
		if self.is_executable() {
			return Err(InstructionError::ExecutableLamportChange);
		}
		// don't touch the account if the lamports do not change
		if self.lamports() == lamports {
			return Ok(());
		}
		self.touch()?;
		self.account.set_lamports(lamports);
		Ok(())
	}

	/// Adds lamports to this account (transaction wide)
	#[cfg(not(target_os = "solana"))]
	pub fn checked_add(&mut self, lamports: Lamports<T>) -> Result<(), InstructionError> {
		self.set_lamports(
			self.lamports()
				.checked_add_lamports(lamports)
				.ok_or(InstructionError::ArithmeticOverflow)?,
		)
	}

	#[cfg(not(target_os = "solana"))]
	pub fn checked_add_lamports(&mut self, lamports: u64) -> Result<(), InstructionError> {
		self.set_lamports(
			self.lamports()
				.checked_add(lamports)
				.ok_or(InstructionError::ArithmeticOverflow)?,
		)
	}

	/// Subtracts lamports from this account (transaction wide)
	#[cfg(not(target_os = "solana"))]
	pub fn checked_sub(&mut self, lamports: Lamports<T>) -> Result<(), InstructionError> {
		self.set_lamports(
			self.lamports()
				.checked_sub_lamports(lamports)
				.ok_or(InstructionError::ArithmeticOverflow)?,
		)
	}

	#[cfg(not(target_os = "solana"))]
	pub fn checked_sub_lamports(&mut self, lamports: u64) -> Result<(), InstructionError> {
		self.set_lamports(
			self.lamports()
				.checked_sub(lamports)
				.ok_or(InstructionError::ArithmeticOverflow)?,
		)
	}

	/// Returns a read-only slice of the account data (transaction wide)
	#[inline]
	pub fn get_data(&self) -> &[u8] {
		self.account.data()
	}

	/// Returns a writable slice of the account data (transaction wide)
	#[cfg(not(target_os = "solana"))]
	pub fn get_data_mut(&mut self) -> Result<&mut [u8], InstructionError> {
		self.can_data_be_changed()?;
		self.touch()?;
		self.make_data_mut();
		Ok(self.account.data_as_mut_slice())
	}

	/// Returns the spare capacity of the vector backing the account data.
	///
	/// This method should only ever be used during CPI, where after a shrinking
	/// realloc we want to zero the spare capacity.
	#[cfg(not(target_os = "solana"))]
	pub fn spare_data_capacity_mut(&mut self) -> Result<&mut [MaybeUninit<u8>], InstructionError> {
		debug_assert!(!self.account.is_shared());
		Ok(self.account.spare_data_capacity_mut())
	}

	/// Overwrites the account data and size (transaction wide).
	///
	/// You should always prefer set_data_from_slice(). Calling this method is
	/// currently safe but requires some special casing during CPI when direct
	/// account mapping is enabled.
	#[cfg(all(not(target_os = "solana"), any(test, feature = "dev-context-only-utils")))]
	pub fn set_data(&mut self, data: Vec<u8>) -> Result<(), InstructionError> {
		self.can_data_be_resized(data.len())?;
		self.can_data_be_changed()?;
		self.touch()?;

		self.update_accounts_resize_delta(data.len())?;
		self.account.set_data(data);
		Ok(())
	}

	/// Overwrites the account data and size (transaction wide).
	///
	/// Call this when you have a slice of data you do not own and want to
	/// replace the account data with it.
	#[cfg(not(target_os = "solana"))]
	pub fn set_data_from_slice(&mut self, data: &[u8]) -> Result<(), InstructionError> {
		self.can_data_be_resized(data.len())?;
		self.can_data_be_changed()?;
		self.touch()?;
		self.update_accounts_resize_delta(data.len())?;
		// Calling make_data_mut() here guarantees that set_data_from_slice()
		// copies in places, extending the account capacity if necessary but
		// never reducing it. This is required as the account migh be directly
		// mapped into a MemoryRegion, and therefore reducing capacity would
		// leave a hole in the vm address space. After CPI or upon program
		// termination, the runtime will zero the extra capacity.
		self.make_data_mut();
		self.account.set_data_from_slice(data);

		Ok(())
	}

	/// Resizes the account data (transaction wide)
	///
	/// Fills it with zeros at the end if is extended or truncates at the end otherwise.
	#[cfg(not(target_os = "solana"))]
	pub fn set_data_length(&mut self, new_length: usize) -> Result<(), InstructionError> {
		self.can_data_be_resized(new_length)?;
		self.can_data_be_changed()?;
		// don't touch the account if the length does not change
		if self.get_data().len() == new_length {
			return Ok(());
		}
		self.touch()?;
		self.update_accounts_resize_delta(new_length)?;
		self.account.resize(new_length, 0);
		Ok(())
	}

	/// Appends all elements in a slice to the account
	#[cfg(not(target_os = "solana"))]
	pub fn extend_from_slice(&mut self, data: &[u8]) -> Result<(), InstructionError> {
		let new_len = self.get_data().len().saturating_add(data.len());
		self.can_data_be_resized(new_len)?;
		self.can_data_be_changed()?;

		if data.is_empty() {
			return Ok(());
		}

		self.touch()?;
		self.update_accounts_resize_delta(new_len)?;
		// Even if extend_from_slice never reduces capacity, still realloc using
		// make_data_mut() if necessary so that we grow the account of the full
		// max realloc length in one go, avoiding smaller reallocations.
		self.make_data_mut();
		self.account.extend_from_slice(data);
		Ok(())
	}

	/// Reserves capacity for at least additional more elements to be inserted
	/// in the given account. Does nothing if capacity is already sufficient.
	#[cfg(not(target_os = "solana"))]
	pub fn reserve(&mut self, additional: usize) -> Result<(), InstructionError> {
		// Note that we don't need to call can_data_be_changed() here nor
		// touch() the account. reserve() only changes the capacity of the
		// memory that holds the account but it doesn't actually change content
		// nor length of the account.
		self.make_data_mut();
		self.account.reserve(additional);

		Ok(())
	}

	/// Returns the number of bytes the account can hold without reallocating.
	#[cfg(not(target_os = "solana"))]
	pub fn capacity(&self) -> usize {
		self.account.capacity()
	}

	/// Returns whether the underlying AccountSharedData is shared.
	///
	/// The data is shared if the account has been loaded from the accounts database and has never
	/// been written to. Writing to an account unshares it.
	///
	/// During account serialization, if an account is shared it'll get mapped as CoW, else it'll
	/// get mapped directly as writable.
	#[cfg(not(target_os = "solana"))]
	pub fn is_shared(&self) -> bool {
		self.account.is_shared()
	}

	#[cfg(not(target_os = "solana"))]
	fn make_data_mut(&mut self) {
		// if the account is still shared, it means this is the first time we're
		// about to write into it. Make the account mutable by copying it in a
		// buffer with MAX_PERMITTED_DATA_INCREASE capacity so that if the
		// transaction reallocs, we don't have to copy the whole account data a
		// second time to fullfill the realloc.
		//
		// NOTE: The account memory region CoW code in bpf_loader::create_vm() implements the same
		// logic and must be kept in sync.
		if self.account.is_shared() {
			self.account.reserve(MAX_PERMITTED_DATA_INCREASE);
		}
	}

	/// Deserializes the account data into a state
	#[cfg(not(target_os = "solana"))]
	pub fn get_state<U: serde::de::DeserializeOwned>(&self) -> Result<U, InstructionError> {
		self.account
			.deserialize_data()
			.map_err(|_| InstructionError::InvalidAccountData)
	}

	/// Serializes a state into the account data
	#[cfg(not(target_os = "solana"))]
	pub fn set_state<U: serde::Serialize>(&mut self, state: &U) -> Result<(), InstructionError> {
		let data = self.get_data_mut()?;
		let serialized_size =
			bincode::serialized_size(state).map_err(|_| InstructionError::GenericError)?;
		if serialized_size > data.len() as u64 {
			return Err(InstructionError::AccountDataTooSmall);
		}
		bincode::serialize_into(&mut *data, state).map_err(|_| InstructionError::GenericError)?;
		Ok(())
	}

	// Returns whether or the lamports currently in the account is sufficient for rent exemption
	// should the data be resized to the given size
	#[cfg(not(target_os = "solana"))]
	pub fn is_rent_exempt_at_data_length(&self, data_length: usize) -> bool {
		self.transaction_context.rent.is_exempt(self.get_lamports(), data_length)
	}

	/// Returns whether this account is executable (transaction wide)
	#[inline]
	pub fn is_executable(&self) -> bool {
		self.account.executable()
	}

	/// Configures whether this account is executable (transaction wide)
	#[cfg(not(target_os = "solana"))]
	pub fn set_executable(&mut self, is_executable: bool) -> Result<(), InstructionError> {
		// To become executable an account must be rent exempt
		if !self
			.transaction_context
			.rent
			.is_exempt(self.get_lamports(), self.get_data().len())
		{
			return Err(InstructionError::ExecutableAccountNotRentExempt);
		}
		// Only the owner can set the executable flag
		if !self.is_owned_by_current_program() {
			return Err(InstructionError::ExecutableModified);
		}
		// and only if the account is writable
		if !self.is_writable() {
			return Err(InstructionError::ExecutableModified);
		}
		// one can not clear the executable flag
		if self.is_executable() && !is_executable {
			return Err(InstructionError::ExecutableModified);
		}
		// don't touch the account if the executable flag does not change
		if self.is_executable() == is_executable {
			return Ok(());
		}
		self.touch()?;
		self.account.set_executable(is_executable);
		Ok(())
	}

	/// Returns the rent epoch of this account (transaction wide)
	#[cfg(not(target_os = "solana"))]
	#[inline]
	pub fn get_rent_epoch(&self) -> u64 {
		self.account.rent_epoch()
	}

	/// Returns whether this account is a signer (instruction wide)
	pub fn is_signer(&self) -> bool {
		if self.index_in_instruction < self.instruction_context.get_number_of_program_accounts() {
			return false;
		}
		self.instruction_context
			.is_instruction_account_signer(
				self.index_in_instruction
					.saturating_sub(self.instruction_context.get_number_of_program_accounts()),
			)
			.unwrap_or_default()
	}

	/// Returns whether this account is writable (instruction wide)
	pub fn is_writable(&self) -> bool {
		if self.index_in_instruction < self.instruction_context.get_number_of_program_accounts() {
			return false;
		}
		self.instruction_context
			.is_instruction_account_writable(
				self.index_in_instruction
					.saturating_sub(self.instruction_context.get_number_of_program_accounts()),
			)
			.unwrap_or_default()
	}

	/// Returns true if the owner of this account is the current `InstructionContext`s last program
	/// (instruction wide)
	pub fn is_owned_by_current_program(&self) -> bool {
		self.instruction_context
			.get_last_program_key(self.transaction_context)
			.map(|key| key == self.get_owner())
			.unwrap_or_default()
	}

	/// Returns an error if the account data can not be mutated by the current program
	#[cfg(not(target_os = "solana"))]
	pub fn can_data_be_changed(&self) -> Result<(), InstructionError> {
		// Only non-executable accounts data can be changed
		if self.is_executable() {
			return Err(InstructionError::ExecutableDataModified);
		}
		// and only if the account is writable
		if !self.is_writable() {
			return Err(InstructionError::ReadonlyDataModified);
		}
		// and only if we are the owner
		if !self.is_owned_by_current_program() {
			return Err(InstructionError::ExternalAccountDataModified);
		}
		Ok(())
	}

	/// Returns an error if the account data can not be resized to the given length
	#[cfg(not(target_os = "solana"))]
	pub fn can_data_be_resized(&self, new_length: usize) -> Result<(), InstructionError> {
		let old_length = self.get_data().len();
		// Only the owner can change the length of the data
		if new_length != old_length && !self.is_owned_by_current_program() {
			return Err(InstructionError::AccountDataSizeChanged);
		}
		// The new length can not exceed the maximum permitted length
		if new_length > MAX_PERMITTED_DATA_LENGTH as usize {
			return Err(InstructionError::InvalidRealloc);
		}
		// The resize can not exceed the per-transaction maximum
		let length_delta = (new_length as i64).saturating_sub(old_length as i64);
		if self.transaction_context.accounts_resize_delta()?.saturating_add(length_delta) >
			MAX_PERMITTED_ACCOUNTS_DATA_ALLOCATIONS_PER_TRANSACTION
		{
			return Err(InstructionError::MaxAccountsDataAllocationsExceeded);
		}
		Ok(())
	}

	#[cfg(not(target_os = "solana"))]
	fn touch(&self) -> Result<(), InstructionError> {
		self.transaction_context.accounts().touch(self.index_in_transaction)
	}

	#[cfg(not(target_os = "solana"))]
	fn update_accounts_resize_delta(&mut self, new_len: usize) -> Result<(), InstructionError> {
		let mut accounts_resize_delta = self
			.transaction_context
			.accounts_resize_delta
			.try_borrow_mut()
			.map_err(|_| InstructionError::GenericError)?;
		*accounts_resize_delta = accounts_resize_delta
			.saturating_add((new_len as i64).saturating_sub(self.get_data().len() as i64));
		Ok(())
	}
}

#[cfg(not(target_os = "solana"))]
fn is_zeroed(buf: &[u8]) -> bool {
	const ZEROS_LEN: usize = 1024;
	const ZEROS: [u8; ZEROS_LEN] = [0; ZEROS_LEN];
	let mut chunks = buf.chunks_exact(ZEROS_LEN);

	#[allow(clippy::indexing_slicing)]
	{
		chunks.all(|chunk| chunk == &ZEROS[..]) &&
			chunks.remainder() == &ZEROS[..chunks.remainder().len()]
	}
}
