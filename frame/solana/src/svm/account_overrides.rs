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

use nostd::collections::BTreeMap;
use solana_sdk::{account::AccountSharedData, pubkey::Pubkey, sysvar};

/// Encapsulates overridden accounts, typically used for transaction simulations
#[derive(Default)]
pub struct AccountOverrides {
	accounts: BTreeMap<Pubkey, AccountSharedData>,
}

impl AccountOverrides {
	/// Insert or remove an account with a given pubkey to/from the list of overrides.
	pub fn set_account(&mut self, pubkey: &Pubkey, account: Option<AccountSharedData>) {
		match account {
			Some(account) => self.accounts.insert(*pubkey, account),
			None => self.accounts.remove(pubkey),
		};
	}

	/// Sets in the slot history
	///
	/// Note: no checks are performed on the correctness of the contained data
	pub fn set_slot_history(&mut self, slot_history: Option<AccountSharedData>) {
		self.set_account(&sysvar::slot_history::id(), slot_history);
	}

	/// Gets the account if it's found in the list of overrides
	pub fn get(&self, pubkey: &Pubkey) -> Option<&AccountSharedData> {
		self.accounts.get(pubkey)
	}
}

#[cfg(test)]
mod test {
	use super::AccountOverrides;
	use solana_sdk::{account::AccountSharedData, pubkey::Pubkey, sysvar};

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
