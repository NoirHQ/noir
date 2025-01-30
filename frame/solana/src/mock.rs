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

use crate as pallet_solana;

use frame_support::{
	derive_impl,
	sp_runtime::{
		traits::{ConstU128, Convert, ConvertBack, IdentityLookup},
		BuildStorage,
	},
};
use solana_sdk::{
	hash::Hash,
	pubkey::Pubkey,
	signature::{Keypair, Signer},
};
use sp_core::{crypto::AccountId32, ed25519::Pair, Pair as _};

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;
	#[runtime::pallet_index(2)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(10)]
	pub type Solana = pallet_solana;
}

type AccountId = <Test as frame_system::Config>::AccountId;
type Blockhash = <Test as frame_system::Config>::Hash;
type Balance = u128;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = frame_system::mocking::MockBlock<Self>;
	type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
}

pub struct AccountIdConversion;
impl Convert<Pubkey, AccountId> for AccountIdConversion {
	fn convert(pubkey: Pubkey) -> AccountId {
		AccountId::new(pubkey.to_bytes())
	}
}

impl ConvertBack<Pubkey, AccountId> for AccountIdConversion {
	fn convert_back(account_id: AccountId) -> Pubkey {
		Pubkey::from(<[u8; 32]>::from(account_id))
	}
}

pub struct HashConversion;
impl Convert<Hash, Blockhash> for HashConversion {
	fn convert(hash: Hash) -> Blockhash {
		Blockhash::from(hash.to_bytes())
	}
}
impl ConvertBack<Hash, Blockhash> for HashConversion {
	fn convert_back(hash: Blockhash) -> Hash {
		Hash::new_from_array(hash.0)
	}
}

#[derive_impl(pallet_solana::config_preludes::TestDefaultConfig)]
impl pallet_solana::Config for Test {
	type AccountIdConversion = AccountIdConversion;
	type HashConversion = HashConversion;
	type Balance = <Self as pallet_balances::Config>::Balance;
	type Currency = Balances;
	type DecimalMultiplier = ConstU128<1_000_000_000>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(Keypair::alice().account_id(), sol_into_balances(10)),
			(Keypair::bob().account_id(), sol_into_balances(10)),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

pub const fn sol_into_lamports(sol: u64) -> u64 {
	sol * 10u64.pow(9)
}

pub const fn sol_into_balances(sol: u64) -> Balance {
	(sol_into_lamports(sol) as Balance) * 10u128.pow(9)
}

pub trait KeypairExt: Sized {
	fn create() -> Self;

	fn get(name: &str) -> Self;

	fn account_id(&self) -> AccountId;

	fn alice() -> Self {
		Self::get("Alice")
	}
	fn bob() -> Self {
		Self::get("Bob")
	}
}

fn pair_to_bytes(pair: Pair) -> [u8; 64] {
	let mut bytes = [0u8; 64];
	bytes[0..32].copy_from_slice(&pair.seed()[..]);
	bytes[32..64].copy_from_slice(&pair.public().0[..]);
	bytes
}

impl KeypairExt for Keypair {
	fn create() -> Self {
		let pair = Pair::generate().0;
		Keypair::from_bytes(&pair_to_bytes(pair)).unwrap()
	}

	fn get(name: &str) -> Self {
		let pair = Pair::from_string(&format!("//{}", name), None).unwrap();
		Keypair::from_bytes(&pair_to_bytes(pair)).unwrap()
	}

	fn account_id(&self) -> AccountId {
		AccountId::new(self.pubkey().to_bytes())
	}
}
