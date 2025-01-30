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

use crate::{Balance, Moment};

/// A number of units.
pub struct Unit(pub u32);

impl Unit {
	/// A unit of base currency.
	pub const fn dollars(&self) -> Balance {
		self.0 as Balance * 1_000_000_000_000_000_000
	}
	#[doc(hidden)]
	pub const fn dollar(&self) -> Balance {
		self.dollars()
	}
	/// One hundredth of a dollar.
	pub const fn cents(&self) -> Balance {
		self.dollars() / 100
	}
	#[doc(hidden)]
	pub const fn cent(&self) -> Balance {
		self.cents()
	}
	/// One thousandth of a cent.
	pub const fn millicents(&self) -> Balance {
		self.cents() / 1_000
	}
	#[doc(hidden)]
	pub const fn millicent(&self) -> Balance {
		self.millicents()
	}
	/// Kibibytes.
	pub const fn kibibytes(&self) -> u32 {
		self.0 * 1024
	}
	#[doc(hidden)]
	pub const fn kibibyte(&self) -> u32 {
		self.kibibytes()
	}
	/// Mebibytes.
	pub const fn mebibytes(&self) -> u32 {
		self.kibibytes() * 1024
	}
	#[doc(hidden)]
	pub const fn mebibyte(&self) -> u32 {
		self.mebibytes()
	}
	/// A day in milliseconds.
	pub const fn days(&self) -> Moment {
		self.hours() * 24
	}
	#[doc(hidden)]
	pub const fn day(&self) -> Moment {
		self.days()
	}
	/// An hour in milliseconds.
	pub const fn hours(&self) -> Moment {
		self.minutes() * 60
	}
	#[doc(hidden)]
	pub const fn hour(&self) -> Moment {
		self.hours()
	}
	/// A minute in milliseconds.
	pub const fn minutes(&self) -> Moment {
		self.seconds() * 60
	}
	#[doc(hidden)]
	pub const fn minute(&self) -> Moment {
		self.minutes()
	}
	/// A second in milliseconds.
	pub const fn seconds(&self) -> Moment {
		self.milliseconds() * 1000
	}
	#[doc(hidden)]
	pub const fn second(&self) -> Moment {
		self.seconds()
	}
	/// A millisecond.
	pub const fn milliseconds(&self) -> Moment {
		self.0 as Moment
	}
	#[doc(hidden)]
	pub const fn millisecond(&self) -> Moment {
		self.milliseconds()
	}
}

#[cfg(test)]
mod tests {
	#![allow(non_upper_case_globals)]
	use super::*;

	pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
	pub const CENTS: Balance = DOLLARS / 100;
	pub const MILLICENTS: Balance = CENTS / 1_000;
	pub const KiB: u32 = 1024;
	pub const MiB: u32 = 1024 * KiB;
	pub const DAYS: Moment = 24 * HOURS;
	pub const HOURS: Moment = 60 * MINUTES;
	pub const MINUTES: Moment = 60 * SECONDS;
	pub const SECONDS: Moment = 1000;
	pub const MILLISECONDS: Moment = 1;

	#[test]
	fn test_units() {
		assert_eq!(Unit(42).dollars(), 42 * DOLLARS);
		assert_eq!(Unit(42).cents(), 42 * CENTS);
		assert_eq!(Unit(42).millicents(), 42 * MILLICENTS);
		assert_eq!(Unit(42).kibibytes(), 42 * KiB);
		assert_eq!(Unit(42).mebibytes(), 42 * MiB);
		assert_eq!(Unit(42).days(), 42 * DAYS);
		assert_eq!(Unit(42).hours(), 42 * HOURS);
		assert_eq!(Unit(42).minutes(), 42 * MINUTES);
		assert_eq!(Unit(42).seconds(), 42 * SECONDS);
		assert_eq!(Unit(42).milliseconds(), 42 * MILLISECONDS);
	}
}
