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
		account::AccountSharedData, lamports::Lamports,
		transaction_processing_callback::TransactionProcessingCallback,
	},
	AccountData, AccountMeta, BalanceOf, Config,
};
use frame_support::traits::{
	fungible::Inspect,
	tokens::{Fortitude::Polite, Preservation::Preserve},
};
use nostd::sync::Arc;
use solana_sdk::{account::Account, native_loader, pubkey::Pubkey};

pub struct Bank<T>(core::marker::PhantomData<T>);

impl<T: Config> TransactionProcessingCallback<T> for Bank<T>
where
	T::AccountId: From<[u8; 32]>,
{
	fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
		let account = T::AccountId::from(account.clone().to_bytes());
		let account = <AccountMeta<T>>::get(account)?;
		owners.iter().position(|entry| account.owner == *entry)
	}

	fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData<T>> {
		let pubkey = T::AccountId::from(pubkey.clone().to_bytes());
		let account = <AccountMeta<T>>::get(&pubkey)?;
		let lamports = Lamports::new(T::Currency::reducible_balance(&pubkey, Preserve, Polite));
		let data = <AccountData<T>>::get(&pubkey);
		Some(AccountSharedData {
			lamports,
			data: match data {
				Some(data) => Arc::new(data.into()),
				None => Arc::new(vec![]),
			},
			owner: account.owner,
			executable: account.executable,
			rent_epoch: account.rent_epoch,
		})
	}

	fn add_builtin_account(&self, name: &str, program_id: &Pubkey) {
		let program_id = T::AccountId::from(program_id.clone().to_bytes());
		let existing_genuine_program = <AccountMeta<T>>::get(program_id).and_then(|account| {
			// it's very unlikely to be squatted at program_id as non-system account because of
			// burden to find victim's pubkey/hash. So, when account.owner is indeed
			// native_loader's, it's safe to assume it's a genuine program.
			if native_loader::check_id(&account.owner) {
				Some(account)
			} else {
				// TODO: burn_and_purge_account(program_id, account);
				None
			}
		});

		// introducing builtin program
		if existing_genuine_program.is_some() {
			// The existing account is sufficient
			return;
		}

		/*
		assert!(
			!self.freeze_started(),
			"Can't change frozen bank by adding not-existing new builtin program ({name}, {program_id}). \
			Maybe, inconsistent program activation is detected on snapshot restore?"
		);

		// Add a bogus executable builtin account, which will be loaded and ignored.
		let account = native_loader::create_loadable_account_with_fields(
			name,
			self.inherit_specially_retained_account_fields(&existing_genuine_program),
		);
		self.store_account_and_update_capitalization(program_id, &account);
		*/
	}
}
