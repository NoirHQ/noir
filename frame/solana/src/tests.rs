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
	hash::Hash,
	message::SimpleAddressLoader,
	reserved_account_keys::ReservedAccountKeys,
	signature::Keypair,
	system_program, system_transaction,
	transaction::{MessageHash, SanitizedTransaction, VersionedTransaction},
};

fn before_each() {
	<AccountMeta<Test>>::insert(
		&Keypair::alice().account_id(),
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
}

#[test]
fn create_account_tx_works() {
	new_test_ext().execute_with(|| {
		before_each();

		let tx = system_transaction::create_account(
			&Keypair::alice(),
			&Keypair::bob(),
			Hash::default(),
			1_000_000_000,
			0,
			&system_program::id(),
		);
		let tx = VersionedTransaction::from(tx);
		let sanitized_tx = SanitizedTransaction::try_create(
			tx,
			MessageHash::Compute,
			None,
			SimpleAddressLoader::Disabled,
			&ReservedAccountKeys::empty_key_set(),
		)
		.expect("valid_transaction");

		let bank = <Bank<Test>>::new(<Slot<Test>>::get());

		assert!(bank.load_execute_and_commit_sanitized_transaction(sanitized_tx).is_ok());
		assert_eq!(
			Balances::total_balance(&Keypair::bob().account_id()),
			1_000_000_000_000_000_000u128
		);
	});
}
