// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
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

use cosmos_sdk_proto::cosmos::tx::v1beta1::Fee;
use nostd::{
	string::{String, ToString},
	vec::Vec,
};
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
