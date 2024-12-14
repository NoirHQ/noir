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
use solana_sdk::account::{Account, InheritableAccountFields, DUMMY_INHERITABLE_ACCOUNT_FIELDS};
pub use solana_sdk::native_loader::*;

/// Create an executable account with the given shared object name.
#[cfg(feature = "std")]
pub fn create_loadable_account_with_fields<T: Config>(
	name: &str,
	(lamports, rent_epoch): InheritableAccountFields,
) -> AccountSharedData<T> {
	AccountSharedData::from(Account {
		lamports,
		owner: id(),
		data: name.as_bytes().to_vec(),
		executable: true,
		rent_epoch,
	})
}

#[cfg(feature = "std")]
pub fn create_loadable_account_for_test<T: Config>(name: &str) -> AccountSharedData<T> {
	create_loadable_account_with_fields(name, DUMMY_INHERITABLE_ACCOUNT_FIELDS)
}
