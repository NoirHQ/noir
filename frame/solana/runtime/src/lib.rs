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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use solana_runtime_api::error::Error;
use solana_sdk::pubkey::Pubkey;

pub fn call<T: pallet_solana::Config>(method: String, params: Vec<u8>) -> Result<Vec<u8>, Error> {
	match method.as_str() {
		"getAccountInfo" => {
			let pubkey =
				serde_json::from_slice::<Pubkey>(&params).map_err(|_| Error::ParseError)?;

			let account = pallet_solana::Pallet::<T>::get_account_info(pubkey);
			let bytes = serde_json::to_vec(&account).map_err(|_| Error::ParseError)?;

			Ok(bytes)
		},
		"getMultipleAccounts" => {
			let pubkeys =
				serde_json::from_slice::<Vec<Pubkey>>(&params).map_err(|_| Error::ParseError)?;

			let accounts = pallet_solana::Pallet::<T>::get_multiple_accounts(pubkeys);
			let bytes = serde_json::to_vec(&accounts).map_err(|_| Error::ParseError)?;

			Ok(bytes)
		},
		"getBalance" => {
			let pubkey =
				serde_json::from_slice::<Pubkey>(&params).map_err(|_| Error::ParseError)?;

			let balance = pallet_solana::Pallet::<T>::get_balance(pubkey);
			let bytes = serde_json::to_vec(&balance).map_err(|_| Error::ParseError)?;

			Ok(bytes)
		},
		_ => return Err(Error::UnsupportedMethod),
	}
}
