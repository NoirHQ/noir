// This file is part of Noir.

// Copyright (c) Anza Maintainers <maintainers@anza.xyz>
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

use solana_sdk::{account::AccountSharedData, nonce_account, pubkey::Pubkey};

pub trait NonceInfo {
	fn address(&self) -> &Pubkey;
	fn account(&self) -> &AccountSharedData;
	fn lamports_per_signature(&self) -> Option<u64>;
	fn fee_payer_account(&self) -> Option<&AccountSharedData>;
}

/// Holds limited nonce info available during transaction checks
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoncePartial {
	address: Pubkey,
	account: AccountSharedData,
}

impl NoncePartial {
	pub fn new(address: Pubkey, account: AccountSharedData) -> Self {
		Self { address, account }
	}
}

impl NonceInfo for NoncePartial {
	fn address(&self) -> &Pubkey {
		&self.address
	}
	fn account(&self) -> &AccountSharedData {
		&self.account
	}
	fn lamports_per_signature(&self) -> Option<u64> {
		nonce_account::lamports_per_signature_of(&self.account)
	}
	fn fee_payer_account(&self) -> Option<&AccountSharedData> {
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use solana_sdk::{
		hash::Hash,
		nonce::state::{
			Data as NonceData, DurableNonce, State as NonceState, Versions as NonceVersions,
		},
		system_program,
	};

	#[test]
	fn test_nonce_info() {
		let nonce_address = Pubkey::new_unique();
		let durable_nonce = DurableNonce::from_blockhash(&Hash::new_unique());
		let lamports_per_signature = 42;
		let nonce_account = AccountSharedData::new_data(
			43,
			&NonceVersions::new(NonceState::Initialized(NonceData::new(
				Pubkey::default(),
				durable_nonce,
				lamports_per_signature,
			))),
			&system_program::id(),
		)
		.unwrap();

		// NoncePartial create + NonceInfo impl
		let partial = NoncePartial::new(nonce_address, nonce_account.clone());
		assert_eq!(*partial.address(), nonce_address);
		assert_eq!(*partial.account(), nonce_account);
		assert_eq!(partial.lamports_per_signature(), Some(lamports_per_signature));
		assert_eq!(partial.fee_payer_account(), None);
	}
}
