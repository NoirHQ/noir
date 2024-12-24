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

use crate::SolanaRuntimeCall;
use nostd::marker::PhantomData;
use pallet_solana::Pubkey;
use solana_inline_spl::token::GenericTokenAccount;
use solana_rpc_client_api::filter::RpcFilterType;
use solana_runtime_api::error::Error;
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
		Ok(pallet_solana::Pallet::<T>::get_multiple_accounts(pubkeys)
			.into_iter()
			.map(|(_, account)| account)
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

		Ok(pallet_solana::Pallet::<T>::get_multiple_accounts(pubkeys)
			.into_iter()
			.filter_map(|(pubkey, account)| account.map(|account| (pubkey, account)))
			.filter(|(_, account)| account.owner == program_id && filter_closure(account))
			.collect())
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
