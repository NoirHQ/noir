// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
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

use crate::config::UiAccountEncoding;
use alloc::string::String;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A duplicate representation of an Account for pretty JSON serialization
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct UiAccount {
	pub lamports: u64,
	pub data: UiAccountData,
	pub owner: String,
	pub executable: bool,
	pub rent_epoch: u64,
	pub space: Option<u64>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum UiAccountData {
	LegacyBinary(String), // Legacy. Retained for RPC backwards compatibility
	// Json(ParsedAccount),
	Binary(String, UiAccountEncoding),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct RpcKeyedAccount {
	pub pubkey: String,
	pub account: UiAccount,
}
