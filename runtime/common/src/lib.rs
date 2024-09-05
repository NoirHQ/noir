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

//! Common implementation across Noir runtimes.

#![cfg_attr(not(feature = "std"), no_std)]

#[allow(non_upper_case_globals)]
pub mod units {
	use noir_core_primitives::{Balance, Moment};

	/// A unit of base currency.
	pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
	/// One hundredth of a dollar.
	pub const CENTS: Balance = DOLLARS / 100;

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
}
