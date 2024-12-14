// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::{
	runtime::{account::AccountSharedData, nonce_account},
	Config,
};
use solana_sdk::pubkey::Pubkey;

pub trait NonceInfo<T: Config> {
	fn address(&self) -> &Pubkey;
	fn account(&self) -> &AccountSharedData<T>;
	fn lamports_per_signature(&self) -> Option<u64>;
	fn fee_payer_account(&self) -> Option<&AccountSharedData<T>>;
}

/// Holds limited nonce info available during transaction checks
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoncePartial<T: Config> {
	address: Pubkey,
	account: AccountSharedData<T>,
}

impl<T: Config> NoncePartial<T> {
	pub fn new(address: Pubkey, account: AccountSharedData<T>) -> Self {
		Self { address, account }
	}
}

impl<T: Config> NonceInfo<T> for NoncePartial<T> {
	fn address(&self) -> &Pubkey {
		&self.address
	}
	fn account(&self) -> &AccountSharedData<T> {
		&self.account
	}
	fn lamports_per_signature(&self) -> Option<u64> {
		nonce_account::lamports_per_signature_of(&self.account)
	}
	fn fee_payer_account(&self) -> Option<&AccountSharedData<T>> {
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::AccountSharedData;
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
