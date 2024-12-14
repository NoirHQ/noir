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

#![allow(unexpected_cfgs)]

pub mod account;
pub mod account_loader;
pub mod account_overrides;
pub mod account_rent_state;
pub mod bank;
pub mod invoke_context;
pub mod lamports;
pub mod loaded_programs;
pub mod message_processor;
pub mod meta;
pub mod native_loader;
pub mod nonce_account;
pub mod nonce_info;
pub mod program_loader;
pub mod rbpf;
pub mod rent_collector;
pub mod rollback_accounts;
pub mod sysvar_cache;
pub mod transaction_account_state_info;
pub mod transaction_context;
pub mod transaction_error_metrics;
pub mod transaction_processing_callback;
pub mod transaction_processor;
pub mod transaction_results;

pub use invoke_context::declare_process_instruction;
pub use lamports::Lamports;
pub use rbpf::declare_builtin_function;
pub use solana_program_runtime::ic_msg;
