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
