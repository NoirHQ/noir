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
};
use alloc::{string::String, vec::Vec};
#[cfg(feature = "scale")]
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "scale")]
use scale_info::TypeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct AccountInfoConfig {
	pub encoding: Option<UiAccountEncoding>,
	pub data_slice: Option<UiDataSliceConfig>,
	pub commitment: Option<CommitmentConfig>,
	pub min_context_slot: Option<Slot>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub enum UiAccountEncoding {
	Binary, // Legacy. Retained for RPC backwards compatibility
	Base58,
	Base64,
	JsonParsed,
	Base64Zstd,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct UiDataSliceConfig {
	pub offset: u64, // usize
	pub length: u64, // usize
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct ProgramAccountsConfig {
	pub filters: Option<Vec<FilterType>>,
	pub account_config: AccountInfoConfig,
	pub with_context: Option<bool>,
	pub sort_results: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub enum FilterType {
	DataSize(u64),
	Memcmp(Memcmp),
	TokenAccountState,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct Memcmp {
	/// Data offset to begin match
	offset: u64, // usize
	/// Bytes, encoded with specified encoding
	bytes: MemcmpEncodedBytes,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub enum MemcmpEncodedBytes {
	Base58(String),
	Base64(String),
	Bytes(Vec<u8>),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub enum TokenAccountsFilter {
	Mint(String),
	ProgramId(String),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct ContextConfig {
	pub commitment: Option<CommitmentConfig>,
	pub min_context_slot: Option<Slot>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct SendTransactionConfig {
	pub skip_preflight: bool,
	pub preflight_commitment: Option<CommitmentLevel>,
	pub encoding: Option<UiTransactionEncoding>,
	pub max_retries: Option<u64>, // Option<usize>
	pub min_context_slot: Option<Slot>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub enum UiTransactionEncoding {
	Binary, // Legacy. Retained for RPC backwards compatibility
	Base64,
	Base58,
	Json,
	JsonParsed,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct SimulateTransactionConfig {
	pub sig_verify: bool,
	pub replace_recent_blockhash: bool,
	pub commitment: Option<CommitmentConfig>,
	pub encoding: Option<UiTransactionEncoding>,
	pub accounts: Option<SimulateTransactionAccountsConfig>,
	pub min_context_slot: Option<Slot>,
	pub inner_instructions: bool,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct SimulateTransactionAccountsConfig {
	pub encoding: Option<UiAccountEncoding>,
	pub addresses: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct EpochConfig {
	pub epoch: Option<Epoch>,
	pub commitment: Option<CommitmentConfig>,
	pub min_context_slot: Option<Slot>,
}
