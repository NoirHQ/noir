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

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use solana_sdk::{clock::Epoch, pubkey::Pubkey};

/// This struct will be backed by mmaped and snapshotted data files.
/// So the data layout must be stable and consistent across the entire cluster!
#[derive(
	Serialize,
	Deserialize,
	Clone,
	Debug,
	Default,
	Eq,
	PartialEq,
	Decode,
	Encode,
	MaxEncodedLen,
	TypeInfo,
)]
#[repr(C)]
pub struct AccountMeta {
	// lamports in the account
	//pub lamports: u64,
	/// the epoch at which this account will next owe rent
	pub rent_epoch: Epoch,
	/// the program that owns this account. If executable, the program that loads this account.
	pub owner: Pubkey,
	/// this account's data contains a loaded program (and is now read-only)
	pub executable: bool,
}
