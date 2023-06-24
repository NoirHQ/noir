// This file is part of Noir.

// Copyright (C) 2023 Haderech Pte. Ltd.
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

//! An account alias.

use codec::{Compact, CompactAs, Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

/// An account alias consisting of human readable string and 4-digit number.
///
/// Human readable part is an alphanumeric ([0-9A-Za-z]) string and the following number is 4-digit
/// decimal number padded with leading zeroes. (0000-9999) Two parts are separated by '#' symbol.
#[cfg_attr(feature = "std", derive(Hash))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub struct AccountName(pub(crate) u128);

impl AccountName {
	/// Create a new instance. Given name and tag number should be valid.
	pub fn new(name: &str, tag: u16) -> Result<Self, ()> {
		// name cannot exceed 14 bytes.
		if name.is_empty() || name.as_bytes().len() > 14 {
			return Err(());
		}
		// name should contain alphanumeric characters.
		if !name.chars().all(char::is_alphanumeric) {
			return Err(());
		}
		// tag should be in the range 0000 to 9999.
		if tag >= 10000 {
			return Err(());
		}

		let mut buf = [0u8; 16];
		buf[14 - name.len()..14].copy_from_slice(name.as_bytes());
		buf[14] = (tag >> 8) as u8;
		buf[15] = tag as u8;

		Ok(Self(u128::from_be_bytes(buf)))
	}
}

impl Into<u128> for AccountName {
	fn into(self) -> u128 {
		self.0
	}
}

impl sp_std::str::FromStr for AccountName {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let v: Vec<&str> = s.split('#').collect();
		// check only one # separator exists.
		if v.len() != 2 {
			return Err(());
		}
		// check tag is leading-zero padded 4-digit number.
		if v[1].len() != 4 {
			return Err(());
		}
		let tag = v[1].parse::<u16>().map_err(|_| ())?;

		Self::new(v[0], tag)
	}
}

#[cfg(feature = "std")]
impl ToString for AccountName {
	fn to_string(&self) -> String {
		let buf = self.0.to_be_bytes();
		let pos = buf.iter().position(|&x| x != 0).unwrap();
		let name = std::str::from_utf8(&buf[pos..14]).unwrap();
		let tag = ((buf[14] as u16) << 8) | (buf[15] as u16);
		let mut s = String::new();
		s.push_str(name);
		s.push('#');
		s.push_str(format!("{:04}", tag).as_str());
		s
	}
}

impl sp_std::fmt::Debug for AccountName {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.to_string())
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
		Ok(())
	}
}

impl CompactAs for AccountName {
	type As = u128;

	fn encode_as(&self) -> &Self::As {
		&self.0
	}

	fn decode_from(x: Self::As) -> Result<Self, codec::Error> {
		Ok(Self(x))
	}
}

impl From<Compact<AccountName>> for AccountName {
	fn from(x: Compact<Self>) -> Self {
		x.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_std::str::FromStr;

	#[test]
	fn parse_account_name() {
		let neg_cases = vec![
			"alice",
			"alice#",
			"0789",
			"#0789",
			"alice#789",
			"abcdefghijklmno#0789",
			"a ice#0789",
			"a|ice#0789",
			"alice#078g",
			"alice#10000",
			"alice##0789",
			"alice#0#789",
		];
		for neg in neg_cases {
			let x = AccountName::from_str(neg);
			assert!(x.is_err());
		}

		let v = 0x616c6963650315_u128;
		let x = AccountName::from_str("alice#0789");
		assert!(x.is_ok());
		let x = x.unwrap();
		assert_eq!(format!("{:?}", x), "alice#0789");
		assert_eq!(x.0, v);
	}

	#[test]
	fn account_name_to_string() {
		let account_name = AccountName(29_384_913_928_000_842u128);
		assert_eq!("hello#5450", account_name.to_string());
		let account_name = AccountName(127_979_077_505_287u128);
		assert_eq!("test#4359", account_name.to_string());
		let account_name = AccountName(27_422_272_635_868_188u128);
		assert_eq!("alice#5148", account_name.to_string());
		let account_name = AccountName(129_445_976_357_405_452_681_402_526_503_014_632_977u128);
		assert_eq!("abcabcabcabcab#3601", account_name.to_string());
		let account_name = AccountName(110_369_760_935_936u128);
		assert_eq!("dave#0000", account_name.to_string());
		let account_name = AccountName(7_952_979_429_530_055_700_424_299_970_569u128);
		assert_eq!("daveisalive#0009", account_name.to_string());
	}
}
