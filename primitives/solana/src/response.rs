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
use alloc::string::{String, ToString};
use core::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response<T> {
	pub context: RpcResponseContext,
	pub value: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcResponseContext {
	pub slot: Slot,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_version: Option<RpcApiVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcApiVersion(semver::Version);

impl core::ops::Deref for RpcApiVersion {
	type Target = semver::Version;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Serialize for RpcApiVersion {
	fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.to_string())
	}
}

impl<'de> Deserialize<'de> for RpcApiVersion {
	fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s: String = Deserialize::deserialize(deserializer)?;
		Ok(RpcApiVersion(semver::Version::from_str(&s).map_err(serde::de::Error::custom)?))
	}
}

/// Wrapper for rpc return types of methods that provide responses both with and without context.
/// Main purpose of this is to fix methods that lack context information in their return type,
/// without breaking backwards compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionalContext<T> {
	Context(Response<T>),
	NoContext(T),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RpcKeyedAccount {
	pub pubkey: String,
	pub account: UiAccount,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RpcBlockhash {
	pub blockhash: String,
	pub last_valid_block_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RpcSimulateTransactionResult {
	pub err: Option<TransactionError>,
	pub logs: Option<Vec<String>>,
	pub accounts: Option<Vec<Option<UiAccount>>>,
	pub units_consumed: Option<u64>,
	pub return_data: Option<UiTransactionReturnData>,
	pub inner_instructions: Option<Vec<UiInnerInstructions>>,
	pub replacement_blockhash: Option<RpcBlockhash>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiTransactionReturnData {
	pub program_id: String,
	pub data: (String, UiReturnDataEncoding),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UiReturnDataEncoding {
	Base64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiInnerInstructions {
	/// Transaction instruction index
	pub index: u8,
	/// List of inner instructions
	pub instructions: Vec<UiInstruction>,
}

/// A duplicate representation of an Instruction for pretty JSON serialization
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum UiInstruction {
	Compiled(UiCompiledInstruction),
	Parsed(UiParsedInstruction),
}

/// A duplicate representation of a CompiledInstruction for pretty JSON serialization
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCompiledInstruction {
	pub program_id_index: u8,
	pub accounts: Vec<u8>,
	pub data: String,
	pub stack_height: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum UiParsedInstruction {
	Parsed(ParsedInstruction),
	PartiallyDecoded(UiPartiallyDecodedInstruction),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedInstruction {
	pub program: String,
	pub program_id: String,
	pub parsed: Value,
	pub stack_height: Option<u32>,
}

/// A partially decoded CompiledInstruction that includes explicit account addresses
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPartiallyDecodedInstruction {
	pub program_id: String,
	pub accounts: Vec<String>,
	pub data: String,
	pub stack_height: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcInflationReward {
	pub epoch: Epoch,
	pub effective_slot: Slot,
	pub amount: u64,            // lamports
	pub post_balance: u64,      // lamports
	pub commission: Option<u8>, // Vote account commission when the reward was credited
}
