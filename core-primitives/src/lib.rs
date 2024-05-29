// This file is part of Noir.

// Copyright (C) 2023 Haderech Pte. Ltd.
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

//! Core Noir types.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::traits::{IdentifyAccount, Verify};

/// An index to a block.
pub type BlockNumber = u32;
/// An instant or duration in time.
pub type Moment = u64;
/// Block header type as expected by this runtime.
pub type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = sp_runtime::generic::Block<Header, sp_runtime::OpaqueExtrinsic>;
/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;
/// Balance of an account.
pub type Balance = u128;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = np_runtime::UniversalSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
/// Index of a transaction in the chain.
pub type Nonce = u32;
/// The type for looking up accounts.
pub type AccountIndex = ();
/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;

/// Basic currency unit.
pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
/// Decimals of currency.
pub const DECIMALS: u8 = 18;
/// Symbol of currency.
pub const SYMBOL: &str = "CDT";
