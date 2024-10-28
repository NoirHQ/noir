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

use crate::{mock::*, *};
use frame_support::{assert_ok, traits::fungible::Inspect};
use np_babel::EthereumAddress;
use sp_core::ecdsa;
use sp_runtime::traits::AccountIdConversion;

fn dev_public() -> ecdsa::Public {
	const_hex::decode_to_array(
		b"02509540919faacf9ab52146c9aa40db68172d83777250b28e4679176e49ccdd9f",
	)
	.unwrap()
	.into()
}

#[test]
fn transfer_to_ethereum_address_works() {
	let account = AccountId::from(dev_public());
	let address = EthereumAddress::from(dev_public());
	let interim = address.into_account_truncating();

	new_test_ext().execute_with(|| {
		assert_ok!(Babel::transfer(
			RuntimeOrigin::signed(alice()),
			None,
			VarAddress::Ethereum(address),
			100
		));
		assert_eq!(Balances::balance(&interim), 100);
		assert_eq!(Balances::balance(&account), 0);

		assert_ok!(UnifyAccount::<Test>::unify_ecdsa(&account));
		assert_eq!(Balances::balance(&interim), 0);
		assert_eq!(Balances::balance(&account), 100);
	});
}

#[cfg(feature = "nostr")]
#[test]
fn transfer_to_nostr_address_works() {
	use core::str::FromStr;
	use np_babel::NostrAddress;

	let account = AccountId::from(dev_public());
	let interim = AccountId::new(dev_public()[1..].try_into().unwrap());
	let address =
		NostrAddress::from_str("npub12z25pyvl4t8e4dfpgmy65sxmdqtjmqmhwfgt9rjx0ytkujwvmk0s2yfk08")
			.unwrap();

	new_test_ext().execute_with(|| {
		assert_ok!(Babel::transfer(
			RuntimeOrigin::signed(alice()),
			None,
			VarAddress::Nostr(address),
			100
		));
		assert_eq!(Balances::balance(&interim), 100);
		assert_eq!(Balances::balance(&account), 0);

		assert_ok!(UnifyAccount::<Test>::unify_ecdsa(&account));
		assert_eq!(Balances::balance(&interim), 0);
		assert_eq!(Balances::balance(&account), 100);
	});
}
