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

use alloc::string::{String, ToString};
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

pub fn amount_to_string(amount: &[cosmos_sdk_proto::cosmos::base::v1beta1::Coin]) -> String {
	let mut ret = "".to_string();
	for (i, coin) in amount.iter().enumerate() {
		ret.push_str(&coin.amount);
		ret.push_str(&coin.denom);
		if i < amount.len() - 1 {
			ret.push(',');
		}
	}
	ret
}

#[cfg(test)]
mod tests {
	use crate::coin::amount_to_string;
	use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;

	#[test]
	fn amount_to_string_test() {
		let mut amounts = Vec::<Coin>::new();
		assert_eq!(amount_to_string(&amounts), "");

		amounts.push(Coin { denom: "uatom".to_string(), amount: "1000".to_string() });
		assert_eq!(amount_to_string(&amounts), "1000uatom");

		amounts.push(Coin { denom: "stake".to_string(), amount: "2000".to_string() });

		assert_eq!(amount_to_string(&amounts), "1000uatom,2000stake");
	}
}
