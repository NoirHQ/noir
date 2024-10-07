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

pub mod traits;

use alloc::{
	string::{String, ToString},
	vec::Vec,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coin {
	pub amount: String,
	pub denom: String,
}

impl From<&cosmos_sdk_proto::cosmos::base::v1beta1::Coin> for Coin {
	fn from(coin: &cosmos_sdk_proto::cosmos::base::v1beta1::Coin) -> Self {
		Self { amount: coin.amount.clone(), denom: coin.denom.clone() }
	}
}

impl traits::Coins for Vec<cosmos_sdk_proto::cosmos::base::v1beta1::Coin> {
	type Error = ();

	fn to_string(&self) -> String {
		let mut ret = "".to_string();
		for (i, coin) in self.iter().enumerate() {
			ret.push_str(&coin.amount);
			ret.push_str(&coin.denom);
			if i < self.len() - 1 {
				ret.push(',');
			}
		}
		ret
	}

	fn amount_of(&self, denom: &str) -> Result<u128, Self::Error> {
		self.iter()
			.find(|coin| coin.denom == denom)
			.ok_or(())?
			.amount
			.parse::<u128>()
			.map_err(|_| ())
	}
}

pub struct DecCoin {
	pub denom: String,
	pub amount: u128,
}

#[cfg(test)]
mod tests {
	use super::traits::Coins;
	use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;

	#[test]
	fn amount_to_string_test() {
		let mut amounts = Vec::<Coin>::new();
		assert_eq!(amounts.to_string(), "");

		amounts.push(Coin { denom: "uatom".to_string(), amount: "1000".to_string() });
		assert_eq!(amounts.to_string(), "1000uatom");

		amounts.push(Coin { denom: "stake".to_string(), amount: "2000".to_string() });

		assert_eq!(amounts.to_string(), "1000uatom,2000stake");
	}
}
