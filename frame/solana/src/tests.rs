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
	mock::*,
	runtime::{bank::Bank, meta::AccountMeta as Meta},
	*,
};

use frame_support::{
	sp_runtime::traits::Convert,
	traits::{fungible::Inspect, Get},
};
use solana_sdk::{
	bpf_loader_upgradeable,
	hash::Hash,
	instruction::{self, Instruction},
	message::SimpleAddressLoader,
	reserved_account_keys::ReservedAccountKeys,
	signature::{Keypair, Signer},
	system_program, system_transaction,
	transaction::{MessageHash, Result, SanitizedTransaction, Transaction, VersionedTransaction},
};

fn before_each() -> Bank<Test> {
	<AccountMeta<Test>>::insert(
		&Keypair::alice().account_id(),
		Meta { rent_epoch: u64::MAX, owner: system_program::id(), executable: false },
	);
	<AccountMeta<Test>>::insert(
		&Keypair::bob().account_id(),
		Meta { rent_epoch: u64::MAX, owner: system_program::id(), executable: false },
	);

	System::set_block_number(2);
	<Slot<Test>>::put(2);
	let blockhash = HashConversion::convert(Hash::default());
	<BlockhashQueue<Test>>::insert(
		blockhash,
		HashInfo {
			fee_calculator: Default::default(),
			hash_index: 1,
			timestamp: <<Test as Config>::GenesisTimestamp as Get<u64>>::get() + 400,
		},
	);

	<Bank<Test>>::new(<Slot<Test>>::get())
}

fn process_transaction(bank: &Bank<Test>, tx: Transaction) -> Result<()> {
	let tx = VersionedTransaction::from(tx);
	let sanitized_tx = SanitizedTransaction::try_create(
		tx,
		MessageHash::Compute,
		None,
		SimpleAddressLoader::Disabled,
		&ReservedAccountKeys::empty_key_set(),
	)
	.expect("Transaction must be sanitized");

	bank.load_execute_and_commit_sanitized_transaction(sanitized_tx)
}

#[test]
fn create_account_tx_should_work() {
	new_test_ext().execute_with(|| {
		let bank = before_each();

		let from = Keypair::alice();
		let to = Keypair::charlie();

		let tx = system_transaction::create_account(
			&from,
			&to,
			Hash::default(),
			1_000_000_000,
			0,
			&system_program::id(),
		);

		assert!(process_transaction(&bank, tx).is_ok());
		assert_eq!(Balances::total_balance(&to.account_id()), 1_000_000_000_000_000_000u128);
	});
}

#[test]
fn deploy_program_tx_should_work() {
	new_test_ext().execute_with(|| {
		let bank = before_each();

		let payer = Keypair::alice();
		let program_keypair = Keypair::get("Program");
		let buffer_keypair = Keypair::get("Buffer");

		let program_data =
			std::fs::read("tests/example-programs/simple-transfer/simple_transfer_program.so")
				.expect("program data");
		let program_account_balance = 1_000_000_000;
		let buffer_account_space = program_data.len();
		let buffer_account_balance = 1_000_000_000;

		let create_buffer = bpf_loader_upgradeable::create_buffer(
			&payer.pubkey(),
			&buffer_keypair.pubkey(),
			&payer.pubkey(),
			buffer_account_balance,
			buffer_account_space,
		)
		.expect("create_buffer instructions");

		let mut tx = Transaction::new_with_payer(&create_buffer, Some(&payer.pubkey()));
		tx.sign(&[&payer, &buffer_keypair], Hash::default());
		assert!(process_transaction(&bank, tx).is_ok());

		let chunk_size = 1024;
		for (i, chunk) in program_data.chunks(chunk_size).enumerate() {
			let write_buffer = bpf_loader_upgradeable::write(
				&buffer_keypair.pubkey(),
				&payer.pubkey(),
				(i * chunk_size) as u32,
				chunk.to_vec(),
			);

			let mut tx = Transaction::new_with_payer(&[write_buffer], Some(&payer.pubkey()));
			tx.sign(&[&payer], Hash::default());
			assert!(process_transaction(&bank, tx).is_ok());
		}

		let deploy_program = bpf_loader_upgradeable::deploy_with_max_program_len(
			&payer.pubkey(),
			&program_keypair.pubkey(),
			&buffer_keypair.pubkey(),
			&payer.pubkey(),
			program_account_balance,
			buffer_account_space,
		)
		.expect("deploy_program instructions");

		let mut tx = Transaction::new_with_payer(&deploy_program, Some(&payer.pubkey()));
		tx.sign(&[&payer, &program_keypair], Hash::default());
		assert!(process_transaction(&bank, tx).is_ok());

		let recipient = Keypair::bob();
		let simple_transfer = Instruction::new_with_bytes(
			program_keypair.pubkey(),
			&1_000_000_000u64.to_be_bytes(),
			vec![
				instruction::AccountMeta::new(payer.pubkey(), true),
				instruction::AccountMeta::new(recipient.pubkey(), false),
				instruction::AccountMeta::new_readonly(system_program::id(), false),
			],
		);
		let mut tx = Transaction::new_with_payer(&[simple_transfer], Some(&payer.pubkey()));
		tx.sign(&[&payer], Hash::default());

		assert!(process_transaction(&bank, tx).is_ok());
		assert_eq!(
			Balances::total_balance(&recipient.account_id()),
			11_000_000_000_000_000_000u128
		);
	});
}
