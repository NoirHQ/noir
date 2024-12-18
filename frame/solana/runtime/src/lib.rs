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

use nostd::{string::String, vec::Vec};
use solana_inline_spl::token::GenericTokenAccount;
use solana_rpc_client_api::filter::RpcFilterType;
use solana_runtime_api::error::Error;
use solana_sdk::{
	account::{Account, ReadableAccount},
	pubkey::Pubkey,
};

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

			let accounts: Vec<Option<Account>> =
				pallet_solana::Pallet::<T>::get_multiple_accounts(pubkeys)
					.into_iter()
					.map(|(_, account)| account)
					.collect();
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
		"getProgramAccounts" => {
			let (program_id, pubkeys, filters) =
				serde_json::from_slice::<(Pubkey, Vec<Pubkey>, Vec<RpcFilterType>)>(&params)
					.map_err(|_| Error::ParseError)?;
			let filter_closure = |account: &Account| {
				filters.iter().all(|filter_type| filter_allows(filter_type, account))
			};

			let accounts = pallet_solana::Pallet::<T>::get_multiple_accounts(pubkeys)
				.into_iter()
				.filter_map(|(pubkey, account)| match account {
					Some(account) => Some((pubkey, account)),
					None => None,
				})
				.filter(|(_, account)| account.owner == program_id && filter_closure(account))
				.collect::<Vec<(Pubkey, Account)>>();
			let bytes = serde_json::to_vec(&accounts).map_err(|_| Error::ParseError)?;

			Ok(bytes)
		},
		"getTransactionCount" => {
			let transaction_count = pallet_solana::Pallet::<T>::get_transaction_count();
			let bytes = serde_json::to_vec(&transaction_count).map_err(|_| Error::ParseError)?;

			Ok(bytes)
		},
		_ => return Err(Error::UnsupportedMethod),
	}
}

pub fn filter_allows(filter: &RpcFilterType, account: &Account) -> bool {
	match filter {
		RpcFilterType::DataSize(size) => account.data().len() as u64 == *size,
		RpcFilterType::Memcmp(compare) => compare.bytes_match(account.data()),
		RpcFilterType::TokenAccountState =>
			solana_inline_spl::token_2022::Account::valid_account_data(account.data()),
	}
}
