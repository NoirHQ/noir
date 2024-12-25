// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
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

#![allow(non_upper_case_globals)]

use crate::{Balance, Moment};

/// A unit of base currency.
pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
/// One hundredth of a dollar.
pub const CENTS: Balance = DOLLARS / 100;
/// One thousandth of a cent.
pub const MILLICENTS: Balance = CENTS / 1_000;

/// Kibibytes.
pub const KiB: u32 = 1024;
/// Mebibytes.
pub const MiB: u32 = 1024 * KiB;

/// A day in milliseconds.
pub const DAYS: Moment = 24 * HOURS;
/// An hour in milliseconds.
pub const HOURS: Moment = 60 * MINUTES;
/// A minute in milliseconds.
pub const MINUTES: Moment = 60 * SECONDS;
/// A second in milliseconds.
pub const SECONDS: Moment = 1000;
/// A millisecond.
pub const MILLISECONDS: Moment = 1;
