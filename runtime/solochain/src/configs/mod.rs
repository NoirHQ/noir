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

use crate::*;

mod address_map;
mod aura;
mod babel;
mod balances;
mod base_fee;
mod ethereum;
mod evm;
mod grandpa;
mod sudo;
mod system;
mod timestamp;
mod transaction_payment;

use common::units::{CENTS, SECONDS};

pub const SLOT_DURATION: Moment = 6 * SECONDS;

pub const MINIMUM_PERIOD: Moment = SLOT_DURATION / 2;

pub const EXISTENTIAL_DEPOSIT: Balance = 1 * CENTS;

pub const BLOCK_HASH_COUNT: BlockNumber = 4096;
