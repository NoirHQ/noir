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

use alloc::{
	string::{String, ToString},
	vec::Vec,
};
use cosmos_sdk_proto::cosmos::tx::v1beta1::Fee;
use pallet_cosmos_types::coin::Coin;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdSignDoc {
	pub account_number: String,
	pub chain_id: String,
	pub fee: StdFee,
	pub memo: String,
	pub msgs: Vec<Value>,
	pub sequence: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdFee {
	pub amount: Vec<Coin>,
	pub gas: String,
}

impl From<Fee> for StdFee {
	fn from(fee: Fee) -> Self {
		Self { amount: fee.amount.iter().map(Into::into).collect(), gas: fee.gas_limit.to_string() }
	}
}

pub trait LegacyMsg {
	const AMINO_NAME: &'static str;

	fn get_sign_bytes(self) -> Value
	where
		Self: Sized + Serialize,
	{
		serde_json::json!({ "type": Self::AMINO_NAME.to_string(), "value": serde_json::to_value(self).unwrap() })
	}
}
