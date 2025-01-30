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

#![allow(clippy::too_many_arguments)]

pub mod account_loader;
pub mod account_overrides;
pub mod account_rent_state;
pub mod message_processor;
pub mod nonce_info;
pub mod program_loader;
pub mod rollback_accounts;
pub mod runtime_config;
pub mod transaction_account_state_info;
pub mod transaction_error_metrics;
pub mod transaction_processing_callback;
pub mod transaction_processor;
pub mod transaction_results;

//#[macro_use]
//extern crate solana_metrics;

#[cfg_attr(feature = "frozen-abi", macro_use)]
#[cfg(feature = "frozen-abi")]
extern crate solana_frozen_abi_macro;
