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
		account_rent_state::RentState,
		transaction_context::{IndexOfAccount, TransactionContext},
	},
	Config,
};
use solana_sdk::{
	account::ReadableAccount, message::SanitizedMessage, native_loader, rent::Rent,
	transaction::Result,
};

#[derive(PartialEq, Debug)]
pub(crate) struct TransactionAccountStateInfo {
	rent_state: Option<RentState>, // None: readonly account
}

impl TransactionAccountStateInfo {
	pub(crate) fn new<T: Config>(
		rent: &Rent,
		transaction_context: &TransactionContext<T>,
		message: &SanitizedMessage,
	) -> Vec<Self> {
		(0..message.account_keys().len())
			.map(|i| {
				let rent_state = if message.is_writable(i) {
					let state = if let Ok(account) =
						transaction_context.get_account_at_index(i as IndexOfAccount)
					{
						let account = account.borrow();

						// Native programs appear to be RentPaying because they carry low lamport
						// balances; however they will never be loaded as writable
						debug_assert!(!native_loader::check_id(account.owner()));

						Some(RentState::from_account(&account, rent))
					} else {
						None
					};
					debug_assert!(
						state.is_some(),
						"message and transaction context out of sync, fatal"
					);
					state
				} else {
					None
				};
				Self { rent_state }
			})
			.collect()
	}

	pub(crate) fn verify_changes<T: Config>(
		pre_state_infos: &[Self],
		post_state_infos: &[Self],
		transaction_context: &TransactionContext<T>,
	) -> Result<()> {
		for (i, (pre_state_info, post_state_info)) in
			pre_state_infos.iter().zip(post_state_infos).enumerate()
		{
			RentState::check_rent_state(
				pre_state_info.rent_state.as_ref(),
				post_state_info.rent_state.as_ref(),
				transaction_context,
				i as IndexOfAccount,
			)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::{
		mock::AccountSharedData,
		runtime::{
			account_rent_state::RentState,
			transaction_account_state_info::TransactionAccountStateInfo,
			transaction_context::TransactionContext,
		},
	};
	use solana_sdk::{
		hash::Hash,
		instruction::CompiledInstruction,
		message::{LegacyMessage, Message, MessageHeader, SanitizedMessage},
		rent::Rent,
		reserved_account_keys::ReservedAccountKeys,
		signature::{Keypair, Signer},
		transaction::TransactionError,
	};

	#[test]
	fn test_new() {
		let rent = Rent::default();
		let key1 = Keypair::new();
		let key2 = Keypair::new();
		let key3 = Keypair::new();
		let key4 = Keypair::new();

		let message = Message {
			account_keys: vec![key2.pubkey(), key1.pubkey(), key4.pubkey()],
			header: MessageHeader::default(),
			instructions: vec![
				CompiledInstruction { program_id_index: 1, accounts: vec![0], data: vec![] },
				CompiledInstruction { program_id_index: 1, accounts: vec![2], data: vec![] },
			],
			recent_blockhash: Hash::default(),
		};

		let sanitized_message = SanitizedMessage::Legacy(LegacyMessage::new(
			message,
			&ReservedAccountKeys::empty_key_set(),
		));

		let transaction_accounts = vec![
			(key1.pubkey(), AccountSharedData::default()),
			(key2.pubkey(), AccountSharedData::default()),
			(key3.pubkey(), AccountSharedData::default()),
		];

		let context = TransactionContext::new(transaction_accounts, rent.clone(), 20, 20);
		let result = TransactionAccountStateInfo::new(&rent, &context, &sanitized_message);
		assert_eq!(
			result,
			vec![
				TransactionAccountStateInfo { rent_state: Some(RentState::Uninitialized) },
				TransactionAccountStateInfo { rent_state: None },
				TransactionAccountStateInfo { rent_state: Some(RentState::Uninitialized) }
			]
		);
	}

	#[test]
	#[should_panic(expected = "message and transaction context out of sync, fatal")]
	fn test_new_panic() {
		let rent = Rent::default();
		let key1 = Keypair::new();
		let key2 = Keypair::new();
		let key3 = Keypair::new();
		let key4 = Keypair::new();

		let message = Message {
			account_keys: vec![key2.pubkey(), key1.pubkey(), key4.pubkey(), key3.pubkey()],
			header: MessageHeader::default(),
			instructions: vec![
				CompiledInstruction { program_id_index: 1, accounts: vec![0], data: vec![] },
				CompiledInstruction { program_id_index: 1, accounts: vec![2], data: vec![] },
			],
			recent_blockhash: Hash::default(),
		};

		let sanitized_message = SanitizedMessage::Legacy(LegacyMessage::new(
			message,
			&ReservedAccountKeys::empty_key_set(),
		));

		let transaction_accounts = vec![
			(key1.pubkey(), AccountSharedData::default()),
			(key2.pubkey(), AccountSharedData::default()),
			(key3.pubkey(), AccountSharedData::default()),
		];

		let context = TransactionContext::new(transaction_accounts, rent.clone(), 20, 20);
		let _result = TransactionAccountStateInfo::new(&rent, &context, &sanitized_message);
	}

	#[test]
	fn test_verify_changes() {
		let key1 = Keypair::new();
		let key2 = Keypair::new();
		let pre_rent_state = vec![
			TransactionAccountStateInfo { rent_state: Some(RentState::Uninitialized) },
			TransactionAccountStateInfo { rent_state: Some(RentState::Uninitialized) },
		];
		let post_rent_state =
			vec![TransactionAccountStateInfo { rent_state: Some(RentState::Uninitialized) }];

		let transaction_accounts = vec![
			(key1.pubkey(), AccountSharedData::default()),
			(key2.pubkey(), AccountSharedData::default()),
		];

		let context = TransactionContext::new(transaction_accounts, Rent::default(), 20, 20);

		let result = TransactionAccountStateInfo::verify_changes(
			&pre_rent_state,
			&post_rent_state,
			&context,
		);
		assert!(result.is_ok());

		let pre_rent_state =
			vec![TransactionAccountStateInfo { rent_state: Some(RentState::Uninitialized) }];
		let post_rent_state = vec![TransactionAccountStateInfo {
			rent_state: Some(RentState::RentPaying { data_size: 2, lamports: 5 }),
		}];

		let transaction_accounts = vec![
			(key1.pubkey(), AccountSharedData::default()),
			(key2.pubkey(), AccountSharedData::default()),
		];

		let context = TransactionContext::new(transaction_accounts, Rent::default(), 20, 20);
		let result = TransactionAccountStateInfo::verify_changes(
			&pre_rent_state,
			&post_rent_state,
			&context,
		);
		assert_eq!(
			result.err(),
			Some(TransactionError::InsufficientFundsForRent { account_index: 0 })
		);
	}
}
