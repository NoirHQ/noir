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

use crate::{error::Error, SolanaRuntimeCall};
use frame_support::traits::Get;
use nostd::marker::PhantomData;
use pallet_solana::Pubkey;
use solana_inline_spl::token::GenericTokenAccount;
use solana_rpc_client_api::filter::RpcFilterType;
use solana_sdk::account::{Account, ReadableAccount};

pub struct AccountInfo<T>(PhantomData<T>);
impl<T> SolanaRuntimeCall<Pubkey, Option<Account>> for AccountInfo<T>
where
	T: pallet_solana::Config,
{
	fn call(pubkey: Pubkey) -> Result<Option<Account>, Error> {
		Ok(pallet_solana::Pallet::<T>::get_account_info(pubkey))
	}
}

pub struct MultipleAccounts<T>(PhantomData<T>);
impl<T> SolanaRuntimeCall<Vec<Pubkey>, Vec<Option<Account>>> for MultipleAccounts<T>
where
	T: pallet_solana::Config,
{
	fn call(pubkeys: Vec<Pubkey>) -> Result<Vec<Option<Account>>, Error> {
		Ok(pubkeys
			.into_iter()
			.map(|pubkey| pallet_solana::Pallet::<T>::get_account_info(pubkey))
			.collect())
	}
}

pub struct ProgramAccounts<T>(PhantomData<T>);
impl<T> SolanaRuntimeCall<(Pubkey, Vec<Pubkey>, Vec<RpcFilterType>), Vec<(Pubkey, Account)>>
	for ProgramAccounts<T>
where
	T: pallet_solana::Config,
{
	fn call(
		(program_id, pubkeys, filters): (Pubkey, Vec<Pubkey>, Vec<RpcFilterType>),
	) -> Result<Vec<(Pubkey, Account)>, Error> {
		let filter_closure = |account: &Account| {
			filters.iter().all(|filter_type| filter_allows(filter_type, account))
		};

		let byte_limit_for_scan =
			T::ScanResultsLimitBytes::get().map(|byte_limit| byte_limit as usize);
		let mut sum: usize = 0;
		let mut accounts = Vec::new();

		for pubkey in pubkeys.iter() {
			if let Some(account) = pallet_solana::Pallet::<T>::get_account_info(*pubkey) {
				if account.owner == program_id && filter_closure(&account) {
					if Self::accumulate_and_check_scan_result_size(
						&mut sum,
						&account,
						byte_limit_for_scan,
					) {
						break;
					}
					accounts.push((*pubkey, account));
				}
			}
		}

		Ok(accounts)
	}
}

impl<T> ProgramAccounts<T> {
	/// Accumulate size of (pubkey + account) into sum.
	/// Return true if sum > 'byte_limit_for_scan'
	fn accumulate_and_check_scan_result_size(
		sum: &mut usize,
		account: &Account,
		byte_limit_for_scan: Option<usize>,
	) -> bool {
		if let Some(byte_limit) = byte_limit_for_scan {
			let added = Self::calc_scan_result_size(account);
			*sum = sum.saturating_add(added);
			*sum > byte_limit
		} else {
			false
		}
	}

	fn calc_scan_result_size(account: &Account) -> usize {
		account.data().len() + std::mem::size_of::<Account>() + std::mem::size_of::<Pubkey>()
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
