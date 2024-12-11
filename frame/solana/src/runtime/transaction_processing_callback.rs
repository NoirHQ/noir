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
use solana_sdk::pubkey::Pubkey;

/// Runtime callbacks for transaction processing.
pub trait TransactionProcessingCallback<T: Config> {
	fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize>;

	fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData<T>>;

	fn add_builtin_account(&self, _name: &str, _program_id: &Pubkey) {}
}
