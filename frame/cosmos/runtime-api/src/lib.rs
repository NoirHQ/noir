// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
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

#![cfg_attr(not(feature = "std"), no_std)]

use nostd::{string::String, vec::Vec};
use pallet_cosmos_types::tx::SimulateResponse;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_api::decl_runtime_apis;
use sp_runtime::traits::Block as BlockT;

#[derive(Clone, Decode, Encode, Debug, Eq, PartialEq, TypeInfo)]
pub enum SimulateError {
	InvalidTransaction,
	InternalError(Vec<u8>),
}

pub type SimulateResult = Result<SimulateResponse, SimulateError>;

#[derive(Clone, Decode, Encode, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ChainInfo {
	pub chain_id: String,
	pub bech32_prefix: String,
	pub name: String,
	pub version: String,
}

decl_runtime_apis! {
	pub trait CosmosRuntimeApi {
		fn convert_tx(tx_bytes: Vec<u8>) -> <Block as BlockT>::Extrinsic;
		fn simulate(tx_bytes: Vec<u8>) -> SimulateResult;
		fn chain_info() -> ChainInfo;
	}
}
