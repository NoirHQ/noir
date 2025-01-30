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

use crate::{mock::*, *};
use frame_support::{assert_ok, traits::fungible::Inspect};
use np_babel::EthereumAddress;
use sp_core::{ecdsa, sha2_256};
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

#[test]
fn ecdsa_verify_prehashed() {
	let signature = const_hex::decode("f7e0d198c62821cc5817c8e935f523308301e29819f5d882f3249b9e173a614f38000ddbff446c0abfa7c7d019dbb17072b28933fc8187c973fbf03d0459f76e").unwrap();
	let message = const_hex::decode("0a93010a90010a1c2f636f736d6f732e62616e6b2e763162657461312e4d736753656e6412700a2d636f736d6f7331716436396e75776a393567746134616b6a677978746a39756a6d7a34773865646d7179737177122d636f736d6f7331676d6a32657861673033747467616670726b6463337438383067726d61396e776566636432771a100a057561746f6d12073130303030303012710a4e0a460a1f2f636f736d6f732e63727970746f2e736563703235366b312e5075624b657912230a21020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a112040a020801121f0a150a057561746f6d120c3838363838303030303030301080c0f1c59495141a1174686574612d746573746e65742d30303120ad8a2e").unwrap();
	let message_hash = sha2_256(&message);
	let public_key =
		const_hex::decode("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1")
			.unwrap();

	new_test_ext().execute_with(|| {
		assert!(pallet_cosmwasm::Pallet::<Test>::do_secp256k1_verify(
			&message_hash,
			&signature,
			&public_key
		));
	});
}
