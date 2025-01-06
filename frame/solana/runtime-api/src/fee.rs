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
use nostd::marker::PhantomData;
use solana_compute_budget::compute_budget_processor::process_compute_budget_instructions;
use solana_sdk::{
	feature_set::{
		include_loaded_accounts_data_size_in_fee_calculation, remove_rounding_in_fee_calculation,
		FeatureSet,
	},
	fee::FeeStructure,
	message::{SanitizedMessage, SanitizedVersionedMessage, SimpleAddressLoader, VersionedMessage},
	reserved_account_keys::ReservedAccountKeys,
};

pub struct FeeForMessage<T>(PhantomData<T>);
impl<T> SolanaRuntimeCall<VersionedMessage, u64> for FeeForMessage<T>
where
	T: pallet_solana::Config,
{
	fn call(message: VersionedMessage) -> Result<u64, Error> {
		let sanitized_versioned_message =
			SanitizedVersionedMessage::try_from(message).map_err(|_| Error::InvalidParams)?;
		// TODO: Get address_loader and reserved_account_keys
		let sanitized_message = SanitizedMessage::try_new(
			sanitized_versioned_message,
			SimpleAddressLoader::Disabled,
			&ReservedAccountKeys::empty_key_set(),
		)
		.map_err(|_| Error::InvalidParams)?;

		// TODO: Get fee_structure, lamports_per_signature and feature_set
		let fee_structure = FeeStructure::default();
		let lamports_per_signature = Default::default();
		let feature_set = FeatureSet::default();

		Ok(fee_structure.calculate_fee(
			&sanitized_message,
			lamports_per_signature,
			&process_compute_budget_instructions(sanitized_message.program_instructions_iter())
				.unwrap_or_default()
				.into(),
			feature_set.is_active(&include_loaded_accounts_data_size_in_fee_calculation::id()),
			feature_set.is_active(&remove_rounding_in_fee_calculation::id()),
		))
	}
}
