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

#![cfg(test)]

use super::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn coinbase_should_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Rewards::coinbase(RuntimeOrigin::none(), vec![(0, 80), (1, 21)]),
			Error::<Test>::InvalidReward
		);
		assert_ok!(Rewards::coinbase(RuntimeOrigin::none(), vec![(0, 80), (1, 20)]));
	});
}

#[test]
#[should_panic(expected = "multiple coinbase not allowed")]
fn multiple_coinbase_should_fail() {
	new_test_ext().execute_with(|| {
		Rewards::insert_coinbase(0, vec![(0, 100)]);
		assert_ok!(Rewards::coinbase(RuntimeOrigin::none(), vec![(0, 100)]));
	});
}
