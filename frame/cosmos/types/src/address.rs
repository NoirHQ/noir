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
