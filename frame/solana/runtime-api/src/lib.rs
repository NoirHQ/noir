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

mod error;

use alloc::{string::String, vec::Vec};
use error::Error;
use np_solana::{
	account::{RpcKeyedAccount, UiAccount},
	commitment_config::CommitmentConfig,
	config::{
		AccountInfoConfig, ContextConfig, EpochConfig, ProgramAccountsConfig,
		SendTransactionConfig, SimulateTransactionConfig, TokenAccountsFilter,
	},
	epoch_info::EpochInfo,
	response::{Blockhash, InflationReward, Response, SimulateTransactionResult},
};
use sp_api::decl_runtime_apis;

decl_runtime_apis! {
	pub trait SolanaRuntimeApi {
		fn get_account_info(
			pubkey_str: String,
			config: Option<AccountInfoConfig>,
		) -> Result<Response<Option<UiAccount>>, Error>;

		fn get_multiple_accounts(
			pubkey_strs: Vec<String>,
			config: Option<AccountInfoConfig>,
		) -> Result<Response<Vec<Option<UiAccount>>>, Error>;

		fn get_program_accounts(
			program_id_str: String,
			config: Option<ProgramAccountsConfig>,
		) -> Result<Response<Vec<RpcKeyedAccount>>, Error>;

		fn get_token_accounts_by_owner(
			owner_str: String,
			token_account_filter: TokenAccountsFilter,
			config: Option<AccountInfoConfig>,
		) -> Result<Response<Vec<RpcKeyedAccount>>, Error>;

		fn get_minimum_balance_for_rent_exemption(
			data_len: u64, // usize
			commitment: Option<CommitmentConfig>,
		) -> Result<u64, Error>;

		fn get_latest_blockhash(config: Option<ContextConfig>) -> Result<Response<Blockhash>, Error>;

		fn send_transaction(
			data: String,
			config: Option<SendTransactionConfig>,
		) -> Result<String, Error>;

		fn simulate_transaction(
			data: String,
			config: Option<SimulateTransactionConfig>,
		) -> Result<Response<SimulateTransactionResult>, Error>;

		fn get_inflation_reward(
			address_strs: Vec<String>,
			config: Option<EpochConfig>,
		) -> Result<Vec<Option<InflationReward>>, Error>;

		fn get_fee_for_message(
			data: String,
			config: Option<ContextConfig>,
		) -> Result<Response<Option<u64>>, Error>;

		fn get_balance(
			pubkey_str: String,
			config: Option<ContextConfig>,
		) -> Result<Response<u64>, Error>;

		fn get_genesis_hash() -> Result<String, Error>;

		fn get_epoch_info(config: Option<ContextConfig>) -> Result<EpochInfo, Error>;

		fn get_transaction_count(config: Option<ContextConfig>) -> Result<u64, Error>;

		fn confirm_transaction(
			signature_str: String,
			commitment: Option<CommitmentConfig>,
		) -> Result<Response<bool>, Error>;
	}
}
