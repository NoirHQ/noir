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
use pallet_solana::{runtime::bank::TransactionSimulationResult, Pubkey};
use solana_runtime_api::error::Error;
use solana_sdk::{
	feature_set::FeatureSet,
	message::{SanitizedMessage, SimpleAddressLoader},
	reserved_account_keys::ReservedAccountKeys,
	transaction::{MessageHash, SanitizedTransaction, VersionedTransaction},
};

pub type AccountRawKeys = (Vec<Pubkey>, Option<(Vec<Pubkey>, Vec<Pubkey>)>);

fn verify_transaction(
	transaction: &SanitizedTransaction,
	feature_set: &FeatureSet,
) -> Result<(), Error> {
	transaction
		.verify()
		.map_err(|_| Error::TransactionSignatureVerificationFailure)?;

	transaction
		.verify_precompiles(feature_set)
		.map_err(|_| Error::TransactionPrecompileVerificationFailure)?;

	Ok(())
}

pub struct SimulateTransaction<T>(PhantomData<T>);
impl<T>
	SolanaRuntimeCall<
		(VersionedTransaction, bool, bool),
		(TransactionSimulationResult, AccountRawKeys),
	> for SimulateTransaction<T>
where
	T: pallet_solana::Config,
{
	fn call(
		(transaction, sig_verify, enable_cpi_recording): (VersionedTransaction, bool, bool),
	) -> Result<(TransactionSimulationResult, AccountRawKeys), Error> {
		let transaction = SanitizedTransaction::try_create(
			transaction,
			MessageHash::Compute,
			None,
			SimpleAddressLoader::Disabled,
			&ReservedAccountKeys::empty_key_set(),
		)
		.map_err(|_| Error::InvalidParams)?;

		// TODO: Get feature_set
		let feature_set = FeatureSet::default();
		if sig_verify {
			verify_transaction(&transaction, &feature_set)?;
		}

		let account_keys = get_account_keys(&transaction.message());
		let simulation_result =
			pallet_solana::Pallet::<T>::simulate_transaction(transaction, enable_cpi_recording);

		Ok((simulation_result, account_keys))
	}
}

pub fn get_account_keys(
	message: &SanitizedMessage,
) -> (Vec<Pubkey>, Option<(Vec<Pubkey>, Vec<Pubkey>)>) {
	match message {
		SanitizedMessage::Legacy(legacy_message) =>
			(legacy_message.message.account_keys.clone(), None),
		SanitizedMessage::V0(loaded_message) => (
			loaded_message.message.account_keys.clone(),
			Some((
				loaded_message.loaded_addresses.writable.clone(),
				loaded_message.loaded_addresses.readonly.clone(),
			)),
		),
	}
}
