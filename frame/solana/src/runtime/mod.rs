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
pub mod invoke_context;
pub mod lamports;
pub mod loaded_programs;
pub mod nonce_account;
pub mod rbpf;
pub mod sysvar_cache;
pub mod transaction_context;

pub use invoke_context::declare_process_instruction;
pub use lamports::Lamports;
pub use rbpf::declare_builtin_function;
pub use solana_program_runtime::ic_msg;
