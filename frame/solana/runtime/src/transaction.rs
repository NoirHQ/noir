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
use pallet_solana::runtime::bank::TransactionSimulationResult;
use solana_runtime_api::error::Error;
use solana_sdk::{
	feature_set::FeatureSet,
	message::SimpleAddressLoader,
	reserved_account_keys::ReservedAccountKeys,
	transaction::{MessageHash, SanitizedTransaction, VersionedTransaction},
};

fn verify_transaction(
	transaction: &SanitizedTransaction,
	feature_set: &FeatureSet,
) -> Result<(), Error> {
	#[allow(clippy::question_mark)]
	if transaction.verify().is_err() {
		return Err(Error::TransactionSignatureVerificationFailure);
	}

	if let Err(_) = transaction.verify_precompiles(feature_set) {
		return Err(Error::TransactionPrecompileVerificationFailure);
	}

	Ok(())
}

pub struct SimulateTransaction<T>(PhantomData<T>);
impl<T> SolanaRuntimeCall<(VersionedTransaction, bool, bool), TransactionSimulationResult>
	for SimulateTransaction<T>
where
	T: pallet_solana::Config,
{
	fn call(
		(transaction, sig_verify, enable_cpi_recording): (VersionedTransaction, bool, bool),
	) -> Result<TransactionSimulationResult, Error> {
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

		Ok(pallet_solana::Pallet::<T>::simulate_transaction(transaction, enable_cpi_recording))
	}
}
