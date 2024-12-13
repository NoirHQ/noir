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

use crate::{
	account::UiAccount,
	clock::{Epoch, Slot},
	transaction_error::TransactionError,
};
use alloc::string::String;
#[cfg(feature = "scale")]
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "scale")]
use scale_info::TypeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response<T> {
	pub context: ResponseContext,
	pub value: T,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseContext {
	pub slot: Slot,
	pub api_version: Option<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blockhash {
	pub blockhash: String,
	pub last_valid_block_height: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulateTransactionResult {
	pub err: Option<TransactionError>,
	pub logs: Option<Vec<String>>,
	pub accounts: Option<Vec<Option<UiAccount>>>,
	pub units_consumed: Option<u64>,
	pub return_data: Option<UiTransactionReturnData>,
	pub inner_instructions: Option<Vec<UiInnerInstructions>>,
	pub replacement_blockhash: Option<Blockhash>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiTransactionReturnData {
	pub program_id: String,
	pub data: (String, UiReturnDataEncoding),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiReturnDataEncoding {
	Base64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiInnerInstructions {
	/// Transaction instruction index
	pub index: u8,
	/// List of inner instructions
	pub instructions: Vec<UiInstruction>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiInstruction {
	Compiled(UiCompiledInstruction),
	// Parsed(UiParsedInstruction),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiCompiledInstruction {
	pub program_id_index: u8,
	pub accounts: Vec<u8>,
	pub data: String,
	pub stack_height: Option<u32>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InflationReward {
	pub epoch: Epoch,
	pub effective_slot: Slot,
	pub amount: u64,            // lamports
	pub post_balance: u64,      // lamports
	pub commission: Option<u8>, // Vote account commission when the reward was credited
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionalContext<T> {
	Context(Response<T>),
	NoContext(T),
}
