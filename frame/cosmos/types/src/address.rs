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

use bech32::DecodeError;
use nostd::{
	string::{String, ToString},
	vec::Vec,
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Error {
	DecodeError(DecodeError),
}

pub const AUTH_ADDRESS_LEN: usize = 20;
pub const CONTRACT_ADDRESS_LEN: usize = 32;

pub fn acc_address_from_bech32(address: &str) -> Result<(String, Vec<u8>), Error> {
	bech32::decode(address)
		.map(|(hrp, data)| (hrp.to_string(), data))
		.map_err(Error::DecodeError)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn acc_address_from_bech32_test() {
		let address = "cosmos1qd69nuwj95gta4akjgyxtj9ujmz4w8edmqysqw";
		let (hrp, address_raw) = acc_address_from_bech32(address).unwrap();
		assert_eq!(hrp, "cosmos");

		let address_raw = const_hex::encode(address_raw);
		assert_eq!(address_raw, "037459f1d22d10bed7b6920865c8bc96c5571f2d");
	}
}
