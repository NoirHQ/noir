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
use solana_compute_budget::compute_budget_processor::process_compute_budget_instructions;
use solana_inline_spl::token::GenericTokenAccount;
use solana_rpc_client_api::filter::RpcFilterType;
use solana_runtime_api::error::Error;
use solana_sdk::{
	account::{Account, ReadableAccount},
	feature_set::{
		include_loaded_accounts_data_size_in_fee_calculation, remove_rounding_in_fee_calculation,
		FeatureSet,
	},
	fee::FeeStructure,
	message::{SanitizedMessage, SanitizedVersionedMessage, SimpleAddressLoader, VersionedMessage},
	pubkey::Pubkey,
	reserved_account_keys::ReservedAccountKeys,
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
				.filter_map(|(pubkey, account)| account.map(|account| (pubkey, account)))
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
		"getFeeForMessage" => {
			let message = serde_json::from_slice::<VersionedMessage>(&params)
				.map_err(|_| Error::ParseError)?;
			let sanitized_versioned_message =
				SanitizedVersionedMessage::try_from(message).map_err(|_| Error::ParseError)?;
			// TODO: Get address_loader and reserved_account_keys
			let sanitized_message = SanitizedMessage::try_new(
				sanitized_versioned_message,
				SimpleAddressLoader::Disabled,
				&ReservedAccountKeys::new_all_activated().active,
			)
			.map_err(|_| Error::ParseError)?;

			// TODO: Get fee_structure, lamports_per_signature and feature_set
			let fee_structure = FeeStructure::default();
			let lamports_per_signature = Default::default();
			let feature_set = FeatureSet::default();

			let fee = fee_structure.calculate_fee(
				&sanitized_message,
				lamports_per_signature,
				&process_compute_budget_instructions(sanitized_message.program_instructions_iter())
					.unwrap_or_default()
					.into(),
				feature_set.is_active(&include_loaded_accounts_data_size_in_fee_calculation::id()),
				feature_set.is_active(&remove_rounding_in_fee_calculation::id()),
			);

			let bytes = serde_json::to_vec(&fee).map_err(|_| Error::ParseError)?;
			Ok(bytes)
		},
		"simulateTransaction" => Ok(Vec::new()),
		_ => Err(Error::UnsupportedMethod),
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
