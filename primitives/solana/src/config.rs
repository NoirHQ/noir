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
	clock::{Epoch, Slot},
	commitment_config::{CommitmentConfig, CommitmentLevel},
	filter::RpcFilterType,
};
use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAccountInfoConfig {
	pub encoding: Option<UiAccountEncoding>,
	pub data_slice: Option<UiDataSliceConfig>,
	#[serde(flatten)]
	pub commitment: Option<CommitmentConfig>,
	pub min_context_slot: Option<Slot>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum UiAccountEncoding {
	Binary, // Legacy. Retained for RPC backwards compatibility
	Base58,
	Base64,
	JsonParsed,
	#[serde(rename = "base64+zstd")]
	Base64Zstd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiDataSliceConfig {
	pub offset: usize,
	pub length: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcProgramAccountsConfig {
	pub filters: Option<Vec<RpcFilterType>>,
	#[serde(flatten)]
	pub account_config: RpcAccountInfoConfig,
	pub with_context: Option<bool>,
	pub sort_results: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RpcTokenAccountsFilter {
	Mint(String),
	ProgramId(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcContextConfig {
	#[serde(flatten)]
	pub commitment: Option<CommitmentConfig>,
	pub min_context_slot: Option<Slot>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcSendTransactionConfig {
	#[serde(default)]
	pub skip_preflight: bool,
	pub preflight_commitment: Option<CommitmentLevel>,
	pub encoding: Option<UiTransactionEncoding>,
	pub max_retries: Option<usize>,
	pub min_context_slot: Option<Slot>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcSimulateTransactionConfig {
	#[serde(default)]
	pub sig_verify: bool,
	#[serde(default)]
	pub replace_recent_blockhash: bool,
	#[serde(flatten)]
	pub commitment: Option<CommitmentConfig>,
	pub encoding: Option<UiTransactionEncoding>,
	pub accounts: Option<RpcSimulateTransactionAccountsConfig>,
	pub min_context_slot: Option<Slot>,
	#[serde(default)]
	pub inner_instructions: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UiTransactionEncoding {
	Binary, // Legacy. Retained for RPC backwards compatibility
	Base64,
	Base58,
	Json,
	JsonParsed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcSimulateTransactionAccountsConfig {
	pub encoding: Option<UiAccountEncoding>,
	pub addresses: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcEpochConfig {
	pub epoch: Option<Epoch>,
	#[serde(flatten)]
	pub commitment: Option<CommitmentConfig>,
	pub min_context_slot: Option<Slot>,
}
