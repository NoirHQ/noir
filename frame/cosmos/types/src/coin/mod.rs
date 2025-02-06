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

pub mod traits;

use nostd::{
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
