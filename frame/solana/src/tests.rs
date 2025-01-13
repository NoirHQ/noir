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

use crate::{mock::*, runtime::bank::Bank, *};

use frame_support::{
	sp_runtime::traits::Convert,
	traits::{
		fungible::{Inspect, Mutate},
		Get,
	},
	BoundedVec,
};
use frame_system::pallet_prelude::BlockNumberFor;
use solana_sdk::{
	bpf_loader, bpf_loader_upgradeable,
	hash::Hash,
	instruction::{self, Instruction},
	message::SimpleAddressLoader,
	program_pack::Pack,
	reserved_account_keys::ReservedAccountKeys,
	signature::{Keypair, Signer},
	system_instruction, system_program, system_transaction,
	transaction::{MessageHash, Result, SanitizedTransaction, Transaction, VersionedTransaction},
};

fn before_each() {
	<AccountMeta<Test>>::insert(
		&Keypair::alice().account_id(),
		AccountMetadata { rent_epoch: u64::MAX, owner: system_program::id(), executable: false },
	);
	<AccountMeta<Test>>::insert(
		&Keypair::bob().account_id(),
		AccountMetadata { rent_epoch: u64::MAX, owner: system_program::id(), executable: false },
	);

	set_block_number(2);

	let blockhash = HashConversion::convert(Hash::default());
	<BlockhashQueue<Test>>::insert(
		blockhash,
		HashInfo {
			fee_calculator: Default::default(),
			hash_index: 1,
			timestamp: <<Test as Config>::GenesisTimestamp as Get<u64>>::get() + 400,
		},
	);
}

fn set_block_number(n: BlockNumberFor<Test>) {
	System::set_block_number(n);
	<Slot<Test>>::put(n);
}

fn mock_bank() -> Bank<Test> {
	Bank::new(<Slot<Test>>::get())
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

fn mock_deploy_program(program_id: &Pubkey, data: Vec<u8>) {
	<Pallet<Test>>::deploy_program(*program_id, data, None).unwrap();

	let who = AccountIdConversion::convert(*program_id);
	Balances::mint_into(&who, sol_into_balances(1)).unwrap();
}

#[test]
fn create_account_tx_should_work() {
	new_test_ext().execute_with(|| {
		before_each();
		let bank = mock_bank();

		let from = Keypair::alice();
		let to = Keypair::get("Account");

		let tx = system_transaction::create_account(
			&from,
			&to,
			Hash::default(),
			sol_into_lamports(1),
			0,
			&system_program::id(),
		);

		assert!(process_transaction(&bank, tx).is_ok());
		assert_eq!(Balances::total_balance(&to.account_id()), sol_into_balances(1));
	});
}

#[test]
fn deploy_program_tx_should_work() {
	new_test_ext().execute_with(|| {
		before_each();
		let bank = mock_bank();

		let payer = Keypair::alice();
		let program_keypair = Keypair::get("Program");
		let buffer_keypair = Keypair::get("Buffer");

		let program_data =
			std::fs::read("tests/example-programs/simple-transfer/simple_transfer_program.so")
				.expect("program data");
		let program_account_balance = sol_into_lamports(1);
		let buffer_account_space = program_data.len();
		let buffer_account_balance = sol_into_lamports(1);

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

		set_block_number(3);
		let bank = mock_bank();

		let recipient = Keypair::bob();
		let simple_transfer = Instruction::new_with_bytes(
			program_keypair.pubkey(),
			&sol_into_lamports(1).to_be_bytes(),
			vec![
				instruction::AccountMeta::new(payer.pubkey(), true),
				instruction::AccountMeta::new(recipient.pubkey(), false),
				instruction::AccountMeta::new_readonly(system_program::id(), false),
			],
		);
		let mut tx = Transaction::new_with_payer(&[simple_transfer], Some(&payer.pubkey()));
		tx.sign(&[&payer], Hash::default());

		assert!(process_transaction(&bank, tx).is_ok());
		assert_eq!(Balances::total_balance(&recipient.account_id()), sol_into_balances(11));
	});
}

#[test]
fn spl_token_program_should_work() {
	new_test_ext().execute_with(|| {
		before_each();
		let bank = mock_bank();

		let authority = Keypair::alice();
		let owner = Keypair::bob();
		let mint = Keypair::get("Mint");

		let token_program_id = Pubkey::parse("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
		let program_data =
			std::fs::read("tests/example-programs/token/token_program.so").expect("program data");

		mock_deploy_program(&token_program_id, program_data);

		let create_account = system_instruction::create_account(
			&authority.pubkey(),
			&mint.pubkey(),
			sol_into_lamports(1),
			82,
			&token_program_id,
		);
		let initialize_mint = spl_token::instruction::initialize_mint2(
			&token_program_id,
			&mint.pubkey(),
			&authority.pubkey(),
			None,
			9,
		)
		.expect("initialize_mint2 instruction");
		let mut tx = Transaction::new_with_payer(
			&[create_account, initialize_mint],
			Some(&authority.pubkey()),
		);
		tx.sign(&[&authority, &mint], Hash::default());
		assert!(process_transaction(&bank, tx).is_ok());

		let account = Keypair::get("Account");
		let create_account = system_instruction::create_account(
			&owner.pubkey(),
			&account.pubkey(),
			sol_into_lamports(1),
			165,
			&token_program_id,
		);
		let initialize_account = spl_token::instruction::initialize_account(
			&token_program_id,
			&account.pubkey(),
			&mint.pubkey(),
			&owner.pubkey(),
		)
		.expect("initialize_account instruction");
		let mint_to = spl_token::instruction::mint_to(
			&token_program_id,
			&mint.pubkey(),
			&account.pubkey(),
			&authority.pubkey(),
			&[],
			sol_into_lamports(1_000),
		)
		.expect("mint_to instruction");
		let mut tx = Transaction::new_with_payer(
			&[create_account, initialize_account, mint_to],
			Some(&owner.pubkey()),
		);
		tx.sign(&[&authority, &account, &owner], Hash::default());
		assert!(process_transaction(&bank, tx).is_ok());

		let state = spl_token::state::Account::unpack_from_slice(&<AccountData<Test>>::get(
			&account.account_id(),
		))
		.expect("token acccount state");
		assert_eq!(state.mint, mint.pubkey());
		assert_eq!(state.owner, owner.pubkey());
		assert_eq!(state.amount, sol_into_lamports(1_000));
	});
}

#[test]
fn filter_duplicated_transaction() {
	new_test_ext().execute_with(|| {
		before_each();
		let bank = mock_bank();

		let from = Keypair::alice();
		let to = Keypair::bob();
		let lamports = 100_000_000;

		let transfer = system_instruction::transfer(&from.pubkey(), &to.pubkey(), lamports);
		let mut transaction = Transaction::new_with_payer(&[transfer], Some(&from.pubkey()));

		let origin = RawOrigin::SolanaTransaction(from.pubkey());
		let versioned_tx: VersionedTransaction = transaction.into();
		assert!(Pallet::<Test>::check_transaction(&versioned_tx).is_ok());

		assert!(Pallet::<Test>::transact(origin.into(), versioned_tx.clone()).is_ok());

		// A duplicated transaction was submitted, causing an error.
		assert!(Pallet::<Test>::check_transaction(&versioned_tx).is_err());
	});
}
