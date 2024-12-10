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

#![allow(clippy::arithmetic_side_effects)]
pub mod system_instruction;
pub mod system_processor;

use crate::{runtime::account::AccountSharedData, Config};
use solana_sdk::{account::ReadableAccount, account_utils::StateMut, nonce, system_program};
pub use system_program::id;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SystemAccountKind {
	System,
	Nonce,
}

pub fn get_system_account_kind<T: Config>(
	account: &AccountSharedData<T>,
) -> Option<SystemAccountKind> {
	if system_program::check_id(account.owner()) {
		if account.data().is_empty() {
			Some(SystemAccountKind::System)
		} else if account.data().len() == nonce::State::size() {
			let nonce_versions: nonce::state::Versions = account.state().ok()?;
			match nonce_versions.state() {
				nonce::State::Uninitialized => None,
				nonce::State::Initialized(_) => Some(SystemAccountKind::Nonce),
			}
		} else {
			None
		}
	} else {
		None
	}
}
