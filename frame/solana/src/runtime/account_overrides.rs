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

use crate::{runtime::account::AccountSharedData, Config};
use nostd::collections::HashMap;
use solana_sdk::{pubkey::Pubkey, sysvar};

/// Encapsulates overridden accounts, typically used for transaction simulations
#[derive_where(Default)]
pub struct AccountOverrides<T: Config> {
	accounts: HashMap<Pubkey, AccountSharedData<T>>,
}

impl<T: Config> AccountOverrides<T> {
	/// Insert or remove an account with a given pubkey to/from the list of overrides.
	pub fn set_account(&mut self, pubkey: &Pubkey, account: Option<AccountSharedData<T>>) {
		match account {
			Some(account) => self.accounts.insert(*pubkey, account),
			None => self.accounts.remove(pubkey),
		};
	}

	/// Sets in the slot history
	///
	/// Note: no checks are performed on the correctness of the contained data
	pub fn set_slot_history(&mut self, slot_history: Option<AccountSharedData<T>>) {
		self.set_account(&sysvar::slot_history::id(), slot_history);
	}

	/// Gets the account if it's found in the list of overrides
	pub fn get(&self, pubkey: &Pubkey) -> Option<&AccountSharedData<T>> {
		self.accounts.get(pubkey)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::mock::AccountSharedData;

	#[test]
	fn test_set_account() {
		let mut accounts = AccountOverrides::default();
		let data = AccountSharedData::default();
		let key = Pubkey::new_unique();
		accounts.set_account(&key, Some(data.clone()));
		assert_eq!(accounts.get(&key), Some(&data));

		accounts.set_account(&key, None);
		assert!(accounts.get(&key).is_none());
	}

	#[test]
	fn test_slot_history() {
		let mut accounts = AccountOverrides::default();
		let data = AccountSharedData::default();

		assert_eq!(accounts.get(&sysvar::slot_history::id()), None);
		accounts.set_slot_history(Some(data.clone()));

		assert_eq!(accounts.get(&sysvar::slot_history::id()), Some(&data));
	}
}
