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
		loaded_programs::{
			LoadProgramMetrics, ProgramCacheEntry, ProgramCacheEntryOwner, ProgramCacheEntryType,
			ProgramRuntimeEnvironment, ProgramRuntimeEnvironments, DELAY_VISIBILITY_SLOT_OFFSET,
		},
		transaction_processing_callback::TransactionProcessingCallback,
	},
	Config,
};
use nostd::sync::Arc;
use solana_sdk::{
	account_utils::StateMut,
	bpf_loader, bpf_loader_deprecated,
	bpf_loader_upgradeable::{self, UpgradeableLoaderState},
	clock::Slot,
	instruction::InstructionError,
	loader_v4::{self, LoaderV4State, LoaderV4Status},
	pubkey::Pubkey,
};

#[derive(Debug)]
pub(crate) enum ProgramAccountLoadResult<T: Config> {
	InvalidAccountData(ProgramCacheEntryOwner),
	ProgramOfLoaderV1(AccountSharedData<T>),
	ProgramOfLoaderV2(AccountSharedData<T>),
	ProgramOfLoaderV3(AccountSharedData<T>, AccountSharedData<T>, Slot),
	ProgramOfLoaderV4(AccountSharedData<T>, Slot),
}

pub(crate) fn load_program_from_bytes<T: Config>(
	load_program_metrics: &mut LoadProgramMetrics,
	programdata: &[u8],
	loader_key: &Pubkey,
	account_size: usize,
	deployment_slot: Slot,
	program_runtime_environment: ProgramRuntimeEnvironment<T>,
	reloading: bool,
) -> core::result::Result<ProgramCacheEntry<T>, Box<dyn core::error::Error>> {
	if reloading {
		// Safety: this is safe because the program is being reloaded in the cache.
		unsafe {
			ProgramCacheEntry::reload(
				loader_key,
				program_runtime_environment.clone(),
				deployment_slot,
				deployment_slot.saturating_add(DELAY_VISIBILITY_SLOT_OFFSET),
				programdata,
				account_size,
				load_program_metrics,
			)
		}
	} else {
		ProgramCacheEntry::new(
			loader_key,
			program_runtime_environment.clone(),
			deployment_slot,
			deployment_slot.saturating_add(DELAY_VISIBILITY_SLOT_OFFSET),
			programdata,
			account_size,
			load_program_metrics,
		)
	}
}

pub(crate) fn load_program_accounts<T: Config, CB: TransactionProcessingCallback<T>>(
	callbacks: &CB,
	pubkey: &Pubkey,
) -> Option<ProgramAccountLoadResult<T>> {
	let program_account = callbacks.get_account_shared_data(pubkey)?;

	if loader_v4::check_id(program_account.owner()) {
		return Some(
			crate::programs::loader_v4::get_state(program_account.data())
				.ok()
				.and_then(|state| {
					(!matches!(state.status, LoaderV4Status::Retracted)).then_some(state.slot)
				})
				.map(|slot| ProgramAccountLoadResult::ProgramOfLoaderV4(program_account, slot))
				.unwrap_or(ProgramAccountLoadResult::InvalidAccountData(
					ProgramCacheEntryOwner::LoaderV4,
				)),
		);
	}

	if bpf_loader_deprecated::check_id(program_account.owner()) {
		return Some(ProgramAccountLoadResult::ProgramOfLoaderV1(program_account));
	}

	if bpf_loader::check_id(program_account.owner()) {
		return Some(ProgramAccountLoadResult::ProgramOfLoaderV2(program_account));
	}

	if let Ok(UpgradeableLoaderState::Program { programdata_address }) = program_account.state() {
		if let Some(programdata_account) = callbacks.get_account_shared_data(&programdata_address) {
			if let Ok(UpgradeableLoaderState::ProgramData { slot, upgrade_authority_address: _ }) =
				programdata_account.state()
			{
				return Some(ProgramAccountLoadResult::ProgramOfLoaderV3(
					program_account,
					programdata_account,
					slot,
				));
			}
		}
	}
	Some(ProgramAccountLoadResult::InvalidAccountData(ProgramCacheEntryOwner::LoaderV3))
}

/// Loads the program with the given pubkey.
///
/// If the account doesn't exist it returns `None`. If the account does exist, it must be a program
/// account (belong to one of the program loaders). Returns `Some(InvalidAccountData)` if the
/// program account is `Closed`, contains invalid data or any of the programdata accounts are
/// invalid.
pub fn load_program_with_pubkey<T: Config, CB: TransactionProcessingCallback<T>>(
	callbacks: &CB,
	environments: &ProgramRuntimeEnvironments<T>,
	pubkey: &Pubkey,
	slot: Slot,
	reload: bool,
) -> Option<Arc<ProgramCacheEntry<T>>> {
	let mut load_program_metrics =
		LoadProgramMetrics { program_id: pubkey.to_string(), ..LoadProgramMetrics::default() };

	let loaded_program = match load_program_accounts(callbacks, pubkey)? {
		ProgramAccountLoadResult::InvalidAccountData(owner) =>
			Ok(ProgramCacheEntry::new_tombstone(slot, owner, ProgramCacheEntryType::Closed)),

		ProgramAccountLoadResult::ProgramOfLoaderV1(program_account) => load_program_from_bytes(
			&mut load_program_metrics,
			program_account.data(),
			program_account.owner(),
			program_account.data().len(),
			0,
			environments.program_runtime_v1.clone(),
			reload,
		)
		.map_err(|_| (0, ProgramCacheEntryOwner::LoaderV1)),

		ProgramAccountLoadResult::ProgramOfLoaderV2(program_account) => load_program_from_bytes(
			&mut load_program_metrics,
			program_account.data(),
			program_account.owner(),
			program_account.data().len(),
			0,
			environments.program_runtime_v1.clone(),
			reload,
		)
		.map_err(|_| (0, ProgramCacheEntryOwner::LoaderV2)),

		ProgramAccountLoadResult::ProgramOfLoaderV3(program_account, programdata_account, slot) =>
			programdata_account
				.data()
				.get(UpgradeableLoaderState::size_of_programdata_metadata()..)
				.ok_or(Box::new(InstructionError::InvalidAccountData).into())
				.and_then(|programdata| {
					load_program_from_bytes(
						&mut load_program_metrics,
						programdata,
						program_account.owner(),
						program_account
							.data()
							.len()
							.saturating_add(programdata_account.data().len()),
						slot,
						environments.program_runtime_v1.clone(),
						reload,
					)
				})
				.map_err(|_| (slot, ProgramCacheEntryOwner::LoaderV3)),

		ProgramAccountLoadResult::ProgramOfLoaderV4(program_account, slot) => program_account
			.data()
			.get(LoaderV4State::program_data_offset()..)
			.ok_or(Box::new(InstructionError::InvalidAccountData).into())
			.and_then(|elf_bytes| {
				load_program_from_bytes(
					&mut load_program_metrics,
					elf_bytes,
					&loader_v4::id(),
					program_account.data().len(),
					slot,
					environments.program_runtime_v2.clone(),
					reload,
				)
			})
			.map_err(|_| (slot, ProgramCacheEntryOwner::LoaderV4)),
	}
	.unwrap_or_else(|(slot, owner)| {
		let env = if let ProgramCacheEntryOwner::LoaderV4 = &owner {
			environments.program_runtime_v2.clone()
		} else {
			environments.program_runtime_v1.clone()
		};
		ProgramCacheEntry::new_tombstone(
			slot,
			owner,
			ProgramCacheEntryType::FailedVerification(env),
		)
	});

	//let mut timings = ExecuteDetailsTimings::default();
	//load_program_metrics.submit_datapoint(&mut timings);
	//loaded_program.update_access_slot(slot);
	Some(Arc::new(loaded_program))
}
