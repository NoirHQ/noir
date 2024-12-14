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
	runtime::{
		account::{AccountSharedData, ReadableAccount},
		nonce_info::{NonceInfo, NoncePartial},
	},
	Config,
};
use solana_sdk::{clock::Epoch, pubkey::Pubkey};

/// Captured account state used to rollback account state for nonce and fee
/// payer accounts after a failed executed transaction.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum RollbackAccounts<T: Config> {
	FeePayerOnly { fee_payer_account: AccountSharedData<T> },
	SameNonceAndFeePayer { nonce: NoncePartial<T> },
	SeparateNonceAndFeePayer { nonce: NoncePartial<T>, fee_payer_account: AccountSharedData<T> },
}

#[cfg(feature = "dev-context-only-utils")]
impl<T: Config> Default for RollbackAccounts<T> {
	fn default() -> Self {
		Self::FeePayerOnly { fee_payer_account: AccountSharedData::default() }
	}
}

impl<T: Config> RollbackAccounts<T> {
	pub fn new(
		nonce: Option<NoncePartial<T>>,
		fee_payer_address: Pubkey,
		mut fee_payer_account: AccountSharedData<T>,
		fee_payer_rent_debit: u64,
		fee_payer_loaded_rent_epoch: Epoch,
	) -> Self {
		// When the fee payer account is rolled back due to transaction failure,
		// rent should not be charged so credit the previously debited rent
		// amount.
		fee_payer_account
			.set_lamports(fee_payer_account.get_lamports().saturating_add(fee_payer_rent_debit));

		if let Some(nonce) = nonce {
			if &fee_payer_address == nonce.address() {
				RollbackAccounts::SameNonceAndFeePayer {
					nonce: NoncePartial::new(fee_payer_address, fee_payer_account),
				}
			} else {
				RollbackAccounts::SeparateNonceAndFeePayer { nonce, fee_payer_account }
			}
		} else {
			// When rolling back failed transactions which don't use nonces, the
			// runtime should not update the fee payer's rent epoch so reset the
			// rollback fee payer acocunt's rent epoch to its originally loaded
			// rent epoch value. In the future, a feature gate could be used to
			// alter this behavior such that rent epoch updates are handled the
			// same for both nonce and non-nonce failed transactions.
			fee_payer_account.set_rent_epoch(fee_payer_loaded_rent_epoch);
			RollbackAccounts::FeePayerOnly { fee_payer_account }
		}
	}

	pub fn nonce(&self) -> Option<&NoncePartial<T>> {
		match self {
			Self::FeePayerOnly { .. } => None,
			Self::SameNonceAndFeePayer { nonce } | Self::SeparateNonceAndFeePayer { nonce, .. } =>
				Some(nonce),
		}
	}

	pub fn fee_payer_account(&self) -> &AccountSharedData<T> {
		match self {
			Self::FeePayerOnly { fee_payer_account } |
			Self::SeparateNonceAndFeePayer { fee_payer_account, .. } => fee_payer_account,
			Self::SameNonceAndFeePayer { nonce } => nonce.account(),
		}
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
	fn test_new_fee_payer_only() {
		let fee_payer_address = Pubkey::new_unique();
		let fee_payer_account = AccountSharedData::new(100, 0, &Pubkey::default());
		let fee_payer_rent_epoch = fee_payer_account.rent_epoch();

		const TEST_RENT_DEBIT: u64 = 1;
		let rent_collected_fee_payer_account = {
			let mut account = fee_payer_account.clone();
			account.set_lamports(fee_payer_account.get_lamports() - TEST_RENT_DEBIT);
			account.set_rent_epoch(fee_payer_rent_epoch + 1);
			account
		};

		let rollback_accounts = RollbackAccounts::new(
			None,
			fee_payer_address,
			rent_collected_fee_payer_account,
			TEST_RENT_DEBIT,
			fee_payer_rent_epoch,
		);

		let expected_fee_payer_account = fee_payer_account;
		match rollback_accounts {
			RollbackAccounts::FeePayerOnly { fee_payer_account } => {
				assert_eq!(expected_fee_payer_account, fee_payer_account);
			},
			_ => panic!("Expected FeePayerOnly variant"),
		}
	}

	#[test]
	fn test_new_same_nonce_and_fee_payer() {
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

		const TEST_RENT_DEBIT: u64 = 1;
		let rent_collected_nonce_account = {
			let mut account = nonce_account.clone();
			account.set_lamports(nonce_account.get_lamports() - TEST_RENT_DEBIT);
			account
		};

		let nonce = NoncePartial::new(nonce_address, rent_collected_nonce_account.clone());
		let rollback_accounts = RollbackAccounts::new(
			Some(nonce),
			nonce_address,
			rent_collected_nonce_account,
			TEST_RENT_DEBIT,
			u64::MAX, // ignored
		);

		match rollback_accounts {
			RollbackAccounts::SameNonceAndFeePayer { nonce } => {
				assert_eq!(nonce.address(), &nonce_address);
				assert_eq!(nonce.account(), &nonce_account);
			},
			_ => panic!("Expected SameNonceAndFeePayer variant"),
		}
	}

	#[test]
	fn test_separate_nonce_and_fee_payer() {
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

		let fee_payer_address = Pubkey::new_unique();
		let fee_payer_account = AccountSharedData::new(44, 0, &Pubkey::default());

		const TEST_RENT_DEBIT: u64 = 1;
		let rent_collected_fee_payer_account = {
			let mut account = fee_payer_account.clone();
			account.set_lamports(fee_payer_account.get_lamports() - TEST_RENT_DEBIT);
			account
		};

		let nonce = NoncePartial::new(nonce_address, nonce_account.clone());
		let rollback_accounts = RollbackAccounts::new(
			Some(nonce),
			fee_payer_address,
			rent_collected_fee_payer_account.clone(),
			TEST_RENT_DEBIT,
			u64::MAX, // ignored
		);

		let expected_fee_payer_account = fee_payer_account;
		match rollback_accounts {
			RollbackAccounts::SeparateNonceAndFeePayer { nonce, fee_payer_account } => {
				assert_eq!(nonce.address(), &nonce_address);
				assert_eq!(nonce.account(), &nonce_account);
				assert_eq!(expected_fee_payer_account, fee_payer_account);
			},
			_ => panic!("Expected SeparateNonceAndFeePayer variant"),
		}
	}
}
