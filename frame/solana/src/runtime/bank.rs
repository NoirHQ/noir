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
		account::AccountSharedData,
		account_loader::{CheckedTransactionDetails, TransactionCheckResult},
		lamports::Lamports,
		nonce_account,
		nonce_info::NoncePartial,
		transaction_processing_callback::TransactionProcessingCallback,
		transaction_processor::LoadAndExecuteSanitizedTransactionOutput,
	},
	AccountData, AccountMeta, BalanceOf, BlockhashQueue, Config, Pallet,
};
use frame_support::{
	sp_runtime::traits::{Convert, One, Saturating},
	traits::{
		fungible::Inspect,
		tokens::{Fortitude::Polite, Preservation::Preserve},
	},
};
use frame_system::pallet_prelude::BlockNumberFor;
use nostd::{marker::PhantomData, sync::Arc};
use solana_sdk::{
	account::Account,
	fee_calculator::FeeCalculator,
	message::SanitizedMessage,
	native_loader,
	nonce::{self, state::DurableNonce, NONCED_TX_MARKER_IX_INDEX},
	pubkey::Pubkey,
	transaction::{SanitizedTransaction, TransactionError},
};

#[derive(Default)]
pub struct Bank<T>(PhantomData<T>);

impl<T: Config> Bank<T> {
	pub fn new() -> Self {
		Self(PhantomData)
	}

	fn load_message_nonce_account(
		&self,
		message: &SanitizedMessage,
	) -> Option<(NoncePartial<T>, nonce::state::Data)> {
		let nonce_address = message.get_durable_nonce()?;
		let nonce_account = self.get_account_shared_data(nonce_address)?;
		let nonce_data =
			nonce_account::verify_nonce_account(&nonce_account, message.recent_blockhash())?;

		let nonce_is_authorized = message
			.get_ix_signers(NONCED_TX_MARKER_IX_INDEX as usize)
			.any(|signer| signer == &nonce_data.authority);
		if !nonce_is_authorized {
			return None;
		}

		Some((NoncePartial::new(*nonce_address, nonce_account), nonce_data))
	}

	fn check_and_load_message_nonce_account(
		&self,
		message: &SanitizedMessage,
		next_durable_nonce: &DurableNonce,
	) -> Option<(NoncePartial<T>, nonce::state::Data)> {
		let nonce_is_advanceable = message.recent_blockhash() != next_durable_nonce.as_hash();
		if nonce_is_advanceable {
			self.load_message_nonce_account(message)
		} else {
			None
		}
	}

	pub fn check_transaction(
		&self,
		tx: &SanitizedTransaction,
		//lock_results: &[Result<()>],
		max_age: BlockNumberFor<T>,
		//error_counters: &mut TransactionErrorMetrics,
	) -> TransactionCheckResult<T> {
		let parent_hash = <frame_system::Pallet<T>>::parent_hash();
		let last_blockhash = T::HashConversion::convert(parent_hash.clone());
		let next_durable_nonce = DurableNonce::from_blockhash(&last_blockhash);

		let recent_blockahsh = tx.message().recent_blockhash();

		if let Some(hash_info) = <Pallet<T>>::get_hash_info_if_valid(&parent_hash, max_age) {
			Ok(CheckedTransactionDetails {
				nonce: None,
				lamports_per_signature: hash_info.lamports_per_signature(),
			})
		} else if let Some((nonce, nonce_data)) =
			self.check_and_load_message_nonce_account(tx.message(), &next_durable_nonce)
		{
			Ok(CheckedTransactionDetails {
				nonce: Some(nonce),
				lamports_per_signature: nonce_data.get_lamports_per_signature(),
			})
		} else {
			Err(TransactionError::BlockhashNotFound)
		}

		// TODO: check status cache?
	}
}

impl<T: Config> TransactionProcessingCallback<T> for Bank<T> {
	fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
		let account = T::AccountIdConversion::convert(account.clone());
		let account = <AccountMeta<T>>::get(account)?;
		owners.iter().position(|entry| account.owner == *entry)
	}

	fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData<T>> {
		let pubkey = T::AccountIdConversion::convert(pubkey.clone());
		let account = <AccountMeta<T>>::get(&pubkey)?;
		let lamports = Lamports::new(T::Currency::reducible_balance(&pubkey, Preserve, Polite));
		let data = <AccountData<T>>::get(&pubkey);
		Some(AccountSharedData {
			lamports,
			data: match data {
				Some(data) => Arc::new(data.into()),
				None => Arc::new(vec![]),
			},
			owner: account.owner,
			executable: account.executable,
			rent_epoch: account.rent_epoch,
		})
	}

	fn add_builtin_account(&self, name: &str, program_id: &Pubkey) {
		let program_id = T::AccountIdConversion::convert(program_id.clone());
		let existing_genuine_program = <AccountMeta<T>>::get(program_id).and_then(|account| {
			// it's very unlikely to be squatted at program_id as non-system account because of
			// burden to find victim's pubkey/hash. So, when account.owner is indeed
			// native_loader's, it's safe to assume it's a genuine program.
			if native_loader::check_id(&account.owner) {
				Some(account)
			} else {
				// TODO: burn_and_purge_account(program_id, account);
				None
			}
		});

		// introducing builtin program
		if existing_genuine_program.is_some() {
			// The existing account is sufficient
			return;
		}

		/*
		assert!(
			!self.freeze_started(),
			"Can't change frozen bank by adding not-existing new builtin program ({name}, {program_id}). \
			Maybe, inconsistent program activation is detected on snapshot restore?"
		);

		// Add a bogus executable builtin account, which will be loaded and ignored.
		let account = native_loader::create_loadable_account_with_fields(
			name,
			self.inherit_specially_retained_account_fields(&existing_genuine_program),
		);
		self.store_account_and_update_capitalization(program_id, &account);
		*/
	}
}
