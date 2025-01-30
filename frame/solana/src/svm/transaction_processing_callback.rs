// This file is part of Noir.

// Copyright (C) Anza Maintainers <maintainers@anza.xyz>
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

use solana_sdk::{account::AccountSharedData, pubkey::Pubkey};

/// Runtime callbacks for transaction processing.
pub trait TransactionProcessingCallback {
	fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize>;

	fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData>;

	fn add_builtin_account(&self, _name: &str, _program_id: &Pubkey) {}
}
