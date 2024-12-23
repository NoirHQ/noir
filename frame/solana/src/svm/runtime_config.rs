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

use solana_compute_budget::compute_budget::ComputeBudget;

#[cfg(all(RUSTC_WITH_SPECIALIZATION, feature = "frozen-abi"))]
impl ::solana_frozen_abi::abi_example::AbiExample for RuntimeConfig {
	fn example() -> Self {
		// RuntimeConfig is not Serialize so just rely on Default.
		RuntimeConfig::default()
	}
}

/// Encapsulates flags that can be used to tweak the runtime behavior.
#[derive(Debug, Default, Clone)]
pub struct RuntimeConfig {
	pub compute_budget: Option<ComputeBudget>,
	pub log_messages_bytes_limit: Option<usize>,
	pub transaction_account_lock_limit: Option<usize>,
}
