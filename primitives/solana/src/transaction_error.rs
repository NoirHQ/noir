// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
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

use crate::instruction_error::InstructionError;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
	/// An account is already being processed in another transaction in a way
	/// that does not support parallelism
	AccountInUse,

	/// A `Pubkey` appears twice in the transaction's `account_keys`.  Instructions can reference
	/// `Pubkey`s more than once but the message must contain a list with no duplicate keys
	AccountLoadedTwice,

	/// Attempt to debit an account but found no record of a prior credit.
	AccountNotFound,

	/// Attempt to load a program that does not exist
	ProgramAccountNotFound,

	/// The from `Pubkey` does not have sufficient balance to pay the fee to schedule the
	/// transaction
	InsufficientFundsForFee,

	/// This account may not be used to pay transaction fees
	InvalidAccountForFee,

	/// The bank has seen this transaction before. This can occur under normal operation
	/// when a UDP packet is duplicated, as a user error from a client not updating
	/// its `recent_blockhash`, or as a double-spend attack.
	AlreadyProcessed,

	/// The bank has not seen the given `recent_blockhash` or the transaction is too old and
	/// the `recent_blockhash` has been discarded.
	BlockhashNotFound,

	/// An error occurred while processing an instruction. The first element of the tuple
	/// indicates the instruction index in which the error occurred.
	InstructionError(u8, InstructionError),

	/// Loader call chain is too deep
	CallChainTooDeep,

	/// Transaction requires a fee but has no signature present
	MissingSignatureForFee,

	/// Transaction contains an invalid account reference
	InvalidAccountIndex,

	/// Transaction did not pass signature verification
	SignatureFailure,

	/// This program may not be used for executing instructions
	InvalidProgramForExecution,

	/// Transaction failed to sanitize accounts offsets correctly
	/// implies that account locks are not taken for this TX, and should
	/// not be unlocked.
	SanitizeFailure,

	ClusterMaintenance,

	/// Transaction processing left an account with an outstanding borrowed reference
	AccountBorrowOutstanding,

	/// Transaction would exceed max Block Cost Limit
	WouldExceedMaxBlockCostLimit,

	/// Transaction version is unsupported
	UnsupportedVersion,

	/// Transaction loads a writable account that cannot be written
	InvalidWritableAccount,

	/// Transaction would exceed max account limit within the block
	WouldExceedMaxAccountCostLimit,

	/// Transaction would exceed account data limit within the block
	WouldExceedAccountDataBlockLimit,

	/// Transaction locked too many accounts
	TooManyAccountLocks,

	/// Address lookup table not found
	AddressLookupTableNotFound,

	/// Attempted to lookup addresses from an account owned by the wrong program
	InvalidAddressLookupTableOwner,

	/// Attempted to lookup addresses from an invalid account
	InvalidAddressLookupTableData,

	/// Address table lookup uses an invalid index
	InvalidAddressLookupTableIndex,

	/// Transaction leaves an account with a lower balance than rent-exempt minimum
	InvalidRentPayingAccount,

	/// Transaction would exceed max Vote Cost Limit
	WouldExceedMaxVoteCostLimit,

	/// Transaction would exceed total account data limit
	WouldExceedAccountDataTotalLimit,

	/// Transaction contains a duplicate instruction that is not allowed
	DuplicateInstruction(u8),

	/// Transaction results in an account with insufficient funds for rent
	InsufficientFundsForRent {
		account_index: u8,
	},

	/// Transaction exceeded max loaded accounts data size cap
	MaxLoadedAccountsDataSizeExceeded,

	/// LoadedAccountsDataSizeLimit set for transaction must be greater than 0.
	InvalidLoadedAccountsDataSizeLimit,

	/// Sanitized transaction differed before/after feature activiation. Needs to be resanitized.
	ResanitizationNeeded,

	/// Program execution is temporarily restricted on an account.
	ProgramExecutionTemporarilyRestricted {
		account_index: u8,
	},

	/// The total balance before the transaction does not equal the total balance after the
	/// transaction
	UnbalancedTransaction,

	/// Program cache hit max limit.
	ProgramCacheHitMaxLimit,
}
