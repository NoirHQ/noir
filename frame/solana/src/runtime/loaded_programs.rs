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

use crate::invoke_context::{BuiltinFunctionWithContext, InvokeContext};
use nostd::{
	collections::HashMap,
	sync::{
		atomic::{AtomicU64, Ordering},
		Arc,
	},
};
pub use solana_program_runtime::loaded_programs::LoadProgramMetrics;
use solana_rbpf::{
	elf::Executable,
	program::{BuiltinProgram, FunctionRegistry},
	verifier::RequisiteVerifier,
	vm::Config,
};
use solana_sdk::{
	bpf_loader, bpf_loader_deprecated, bpf_loader_upgradeable,
	clock::{Epoch, Slot},
	loader_v4, native_loader,
	pubkey::Pubkey,
};

pub type ProgramRuntimeEnvironment<T> = Arc<BuiltinProgram<InvokeContext<'static, T>>>;
pub const MAX_LOADED_ENTRY_COUNT: usize = 512;
pub const DELAY_VISIBILITY_SLOT_OFFSET: Slot = 1;

/// The owner of a programs accounts, thus the loader of a program
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProgramCacheEntryOwner {
	#[default]
	NativeLoader,
	LoaderV1,
	LoaderV2,
	LoaderV3,
	LoaderV4,
}

impl TryFrom<&Pubkey> for ProgramCacheEntryOwner {
	type Error = ();
	fn try_from(loader_key: &Pubkey) -> Result<Self, ()> {
		if native_loader::check_id(loader_key) {
			Ok(ProgramCacheEntryOwner::NativeLoader)
		} else if bpf_loader_deprecated::check_id(loader_key) {
			Ok(ProgramCacheEntryOwner::LoaderV1)
		} else if bpf_loader::check_id(loader_key) {
			Ok(ProgramCacheEntryOwner::LoaderV2)
		} else if bpf_loader_upgradeable::check_id(loader_key) {
			Ok(ProgramCacheEntryOwner::LoaderV3)
		} else if loader_v4::check_id(loader_key) {
			Ok(ProgramCacheEntryOwner::LoaderV4)
		} else {
			Err(())
		}
	}
}

impl From<ProgramCacheEntryOwner> for Pubkey {
	fn from(program_cache_entry_owner: ProgramCacheEntryOwner) -> Self {
		match program_cache_entry_owner {
			ProgramCacheEntryOwner::NativeLoader => native_loader::id(),
			ProgramCacheEntryOwner::LoaderV1 => bpf_loader_deprecated::id(),
			ProgramCacheEntryOwner::LoaderV2 => bpf_loader::id(),
			ProgramCacheEntryOwner::LoaderV3 => bpf_loader_upgradeable::id(),
			ProgramCacheEntryOwner::LoaderV4 => loader_v4::id(),
		}
	}
}

/*
	The possible ProgramCacheEntryType transitions:

	DelayVisibility is special in that it is never stored in the cache.
	It is only returned by ProgramCacheForTxBatch::find() when a Loaded entry
	is encountered which is not effective yet.

	Builtin re/deployment:
	- Empty => Builtin in TransactionBatchProcessor::add_builtin
	- Builtin => Builtin in TransactionBatchProcessor::add_builtin

	Un/re/deployment (with delay and cooldown):
	- Empty / Closed => Loaded in UpgradeableLoaderInstruction::DeployWithMaxDataLen
	- Loaded / FailedVerification => Loaded in UpgradeableLoaderInstruction::Upgrade
	- Loaded / FailedVerification => Closed in UpgradeableLoaderInstruction::Close

	Eviction and unloading (in the same slot):
	- Unloaded => Loaded in ProgramCache::assign_program
	- Loaded => Unloaded in ProgramCache::unload_program_entry

	At epoch boundary (when feature set and environment changes):
	- Loaded => FailedVerification in Bank::_new_from_parent
	- FailedVerification => Loaded in Bank::_new_from_parent

	Through pruning (when on orphan fork or overshadowed on the rooted fork):
	- Closed / Unloaded / Loaded / Builtin => Empty in ProgramCache::prune
*/

/// Actual payload of [ProgramCacheEntry].
#[derive(Default)]
pub enum ProgramCacheEntryType<T: crate::Config> {
	/// Tombstone for programs which currently do not pass the verifier but could if the feature
	/// set changed.
	FailedVerification(ProgramRuntimeEnvironment<T>),
	/// Tombstone for programs that were either explicitly closed or never deployed.
	///
	/// It's also used for accounts belonging to program loaders, that don't actually contain
	/// program code (e.g. buffer accounts for LoaderV3 programs).
	#[default]
	Closed,
	/// Tombstone for programs which have recently been modified but the new version is not visible
	/// yet.
	DelayVisibility,
	/// Successfully verified but not currently compiled.
	///
	/// It continues to track usage statistics even when the compiled executable of the program is
	/// evicted from memory.
	Unloaded(ProgramRuntimeEnvironment<T>),
	/// Verified and compiled program
	Loaded(Executable<InvokeContext<'static, T>>),
	/// A built-in program which is not stored on-chain but backed into and distributed with the
	/// validator
	Builtin(BuiltinProgram<InvokeContext<'static, T>>),
}

impl<T: crate::Config> core::fmt::Debug for ProgramCacheEntryType<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			ProgramCacheEntryType::FailedVerification(_) => {
				write!(f, "ProgramCacheEntryType::FailedVerification")
			},
			ProgramCacheEntryType::Closed => write!(f, "ProgramCacheEntryType::Closed"),
			ProgramCacheEntryType::DelayVisibility => {
				write!(f, "ProgramCacheEntryType::DelayVisibility")
			},
			ProgramCacheEntryType::Unloaded(_) => write!(f, "ProgramCacheEntryType::Unloaded"),
			ProgramCacheEntryType::Loaded(_) => write!(f, "ProgramCacheEntryType::Loaded"),
			ProgramCacheEntryType::Builtin(_) => write!(f, "ProgramCacheEntryType::Builtin"),
		}
	}
}

impl<T: crate::Config> ProgramCacheEntryType<T> {
	/// Returns a reference to its environment if it has one
	pub fn get_environment(&self) -> Option<&ProgramRuntimeEnvironment<T>> {
		match self {
			ProgramCacheEntryType::Loaded(program) => Some(program.get_loader()),
			ProgramCacheEntryType::FailedVerification(env) |
			ProgramCacheEntryType::Unloaded(env) => Some(env),
			_ => None,
		}
	}
}

/// Holds a program version at a specific address and on a specific slot / fork.
///
/// It contains the actual program in [ProgramCacheEntryType] and a bunch of meta-data.
#[derive(Debug, Default)]
pub struct ProgramCacheEntry<T: crate::Config> {
	/// The program of this entry
	pub program: ProgramCacheEntryType<T>,
	/// The loader of this entry
	pub account_owner: ProgramCacheEntryOwner,
	/// Size of account that stores the program and program data
	pub account_size: usize,
	/// Slot in which the program was (re)deployed
	pub deployment_slot: Slot,
	/// Slot in which this entry will become active (can be in the future)
	pub effective_slot: Slot,
	/// How often this entry was used by a transaction
	pub tx_usage_counter: AtomicU64,
	/// How often this entry was used by an instruction
	pub ix_usage_counter: AtomicU64,
	/// Latest slot in which the entry was used
	pub latest_access_slot: AtomicU64,
}

impl<T: crate::Config> PartialEq for ProgramCacheEntry<T> {
	fn eq(&self, other: &Self) -> bool {
		self.effective_slot == other.effective_slot &&
			self.deployment_slot == other.deployment_slot &&
			self.is_tombstone() == other.is_tombstone()
	}
}

impl<T: crate::Config> ProgramCacheEntry<T> {
	/// Creates a new user program
	pub fn new(
		loader_key: &Pubkey,
		program_runtime_environment: ProgramRuntimeEnvironment<T>,
		deployment_slot: Slot,
		effective_slot: Slot,
		elf_bytes: &[u8],
		account_size: usize,
		metrics: &mut LoadProgramMetrics,
	) -> Result<Self, Box<dyn core::error::Error>> {
		Self::new_internal(
			loader_key,
			program_runtime_environment,
			deployment_slot,
			effective_slot,
			elf_bytes,
			account_size,
			metrics,
			false, /* reloading */
		)
	}

	/// Reloads a user program, *without* running the verifier.
	///
	/// # Safety
	///
	/// This method is unsafe since it assumes that the program has already been verified. Should
	/// only be called when the program was previously verified and loaded in the cache, but was
	/// unloaded due to inactivity. It should also be checked that the `program_runtime_environment`
	/// hasn't changed since it was unloaded.
	pub unsafe fn reload(
		loader_key: &Pubkey,
		program_runtime_environment: Arc<BuiltinProgram<InvokeContext<'static, T>>>,
		deployment_slot: Slot,
		effective_slot: Slot,
		elf_bytes: &[u8],
		account_size: usize,
		metrics: &mut LoadProgramMetrics,
	) -> Result<Self, Box<dyn core::error::Error>> {
		Self::new_internal(
			loader_key,
			program_runtime_environment,
			deployment_slot,
			effective_slot,
			elf_bytes,
			account_size,
			metrics,
			true, /* reloading */
		)
	}

	fn new_internal(
		loader_key: &Pubkey,
		program_runtime_environment: Arc<BuiltinProgram<InvokeContext<'static, T>>>,
		deployment_slot: Slot,
		effective_slot: Slot,
		elf_bytes: &[u8],
		account_size: usize,
		metrics: &mut LoadProgramMetrics,
		reloading: bool,
	) -> Result<Self, Box<dyn core::error::Error>> {
		//let load_elf_time = Measure::start("load_elf_time");
		// The following unused_mut exception is needed for architectures that do not
		// support JIT compilation.
		#[allow(unused_mut)]
		let mut executable = Executable::load(elf_bytes, program_runtime_environment.clone())?;
		//metrics.load_elf_us = load_elf_time.end_as_us();

		if !reloading {
			//let verify_code_time = Measure::start("verify_code_time");
			executable.verify::<RequisiteVerifier>()?;
			//metrics.verify_code_us = verify_code_time.end_as_us();
		}

		#[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
		{
			//let jit_compile_time = Measure::start("jit_compile_time");
			executable.jit_compile()?;
			//metrics.jit_compile_us = jit_compile_time.end_as_us();
		}

		Ok(Self {
			deployment_slot,
			account_owner: ProgramCacheEntryOwner::try_from(loader_key).unwrap(),
			account_size,
			effective_slot,
			tx_usage_counter: AtomicU64::new(0),
			program: ProgramCacheEntryType::Loaded(executable),
			ix_usage_counter: AtomicU64::new(0),
			latest_access_slot: AtomicU64::new(0),
		})
	}

	pub fn to_unloaded(&self) -> Option<Self> {
		match &self.program {
			ProgramCacheEntryType::Loaded(_) => {},
			ProgramCacheEntryType::FailedVerification(_) |
			ProgramCacheEntryType::Closed |
			ProgramCacheEntryType::DelayVisibility |
			ProgramCacheEntryType::Unloaded(_) |
			ProgramCacheEntryType::Builtin(_) => {
				return None;
			},
		}
		Some(Self {
			program: ProgramCacheEntryType::Unloaded(self.program.get_environment()?.clone()),
			account_owner: self.account_owner,
			account_size: self.account_size,
			deployment_slot: self.deployment_slot,
			effective_slot: self.effective_slot,
			tx_usage_counter: AtomicU64::new(self.tx_usage_counter.load(Ordering::Relaxed)),
			ix_usage_counter: AtomicU64::new(self.ix_usage_counter.load(Ordering::Relaxed)),
			latest_access_slot: AtomicU64::new(self.latest_access_slot.load(Ordering::Relaxed)),
		})
	}

	/// Creates a new built-in program
	pub fn new_builtin(
		deployment_slot: Slot,
		account_size: usize,
		builtin_function: BuiltinFunctionWithContext<T>,
	) -> Self {
		let mut function_registry = FunctionRegistry::default();
		function_registry
			.register_function_hashed(*b"entrypoint", builtin_function)
			.unwrap();
		Self {
			deployment_slot,
			account_owner: ProgramCacheEntryOwner::NativeLoader,
			account_size,
			effective_slot: deployment_slot,
			tx_usage_counter: AtomicU64::new(0),
			program: ProgramCacheEntryType::Builtin(BuiltinProgram::new_builtin(function_registry)),
			ix_usage_counter: AtomicU64::new(0),
			latest_access_slot: AtomicU64::new(0),
		}
	}

	pub fn new_tombstone(
		slot: Slot,
		account_owner: ProgramCacheEntryOwner,
		reason: ProgramCacheEntryType<T>,
	) -> Self {
		let tombstone = Self {
			program: reason,
			account_owner,
			account_size: 0,
			deployment_slot: slot,
			effective_slot: slot,
			tx_usage_counter: AtomicU64::default(),
			ix_usage_counter: AtomicU64::default(),
			latest_access_slot: AtomicU64::new(0),
		};
		debug_assert!(tombstone.is_tombstone());
		tombstone
	}

	pub fn is_tombstone(&self) -> bool {
		matches!(
			self.program,
			ProgramCacheEntryType::FailedVerification(_) |
				ProgramCacheEntryType::Closed |
				ProgramCacheEntryType::DelayVisibility
		)
	}

	fn is_implicit_delay_visibility_tombstone(&self, slot: Slot) -> bool {
		!matches!(self.program, ProgramCacheEntryType::Builtin(_)) &&
			self.effective_slot.saturating_sub(self.deployment_slot) ==
				DELAY_VISIBILITY_SLOT_OFFSET &&
			slot >= self.deployment_slot &&
			slot < self.effective_slot
	}

	pub fn update_access_slot(&self, slot: Slot) {
		let _ = self.latest_access_slot.fetch_max(slot, Ordering::Relaxed);
	}

	pub fn decayed_usage_counter(&self, now: Slot) -> u64 {
		let last_access = self.latest_access_slot.load(Ordering::Relaxed);
		// Shifting the u64 value for more than 63 will cause an overflow.
		let decaying_for = core::cmp::min(63, now.saturating_sub(last_access));
		self.tx_usage_counter.load(Ordering::Relaxed) >> decaying_for
	}

	pub fn account_owner(&self) -> Pubkey {
		self.account_owner.into()
	}
}

/// Globally shared RBPF config and syscall registry
///
/// This is only valid in an epoch range as long as no feature affecting RBPF is activated.
#[derive(Clone, Debug)]
pub struct ProgramRuntimeEnvironments<T: crate::Config> {
	/// For program runtime V1
	pub program_runtime_v1: ProgramRuntimeEnvironment<T>,
	/// For program runtime V2
	pub program_runtime_v2: ProgramRuntimeEnvironment<T>,
}

impl<T: crate::Config> Default for ProgramRuntimeEnvironments<T> {
	fn default() -> Self {
		let empty_loader =
			Arc::new(BuiltinProgram::new_loader(Config::default(), FunctionRegistry::default()));
		Self { program_runtime_v1: empty_loader.clone(), program_runtime_v2: empty_loader }
	}
}

/// Local view into [ProgramCache] which was extracted for a specific TX batch.
///
/// This isolation enables the global [ProgramCache] to continue to evolve (e.g. evictions),
/// while the TX batch is guaranteed it will continue to find all the programs it requires.
/// For program management instructions this also buffers them before they are merged back into the
/// global [ProgramCache].
#[derive(Clone, Debug)]
#[derive_where(Default)]
pub struct ProgramCacheForTxBatch<T: crate::Config> {
	/// Pubkey is the address of a program.
	/// ProgramCacheEntry is the corresponding program entry valid for the slot in which a
	/// transaction is being executed.
	entries: HashMap<Pubkey, Arc<ProgramCacheEntry<T>>>,
	/// Program entries modified during the transaction batch.
	modified_entries: HashMap<Pubkey, Arc<ProgramCacheEntry<T>>>,
	slot: Slot,
	pub environments: ProgramRuntimeEnvironments<T>,
	/// Anticipated replacement for `environments` at the next epoch.
	///
	/// This is `None` during most of an epoch, and only `Some` around the boundaries (at the end
	/// and beginning of an epoch). More precisely, it starts with the cache preparation phase a
	/// few hundred slots before the epoch boundary, and it ends with the first rerooting after
	/// the epoch boundary. Needed when a program is deployed at the last slot of an epoch,
	/// becomes effective in the next epoch. So needs to be compiled with the environment for the
	/// next epoch.
	pub upcoming_environments: Option<ProgramRuntimeEnvironments<T>>,
	/// The epoch of the last rerooting
	pub latest_root_epoch: Epoch,
	pub hit_max_limit: bool,
	pub loaded_missing: bool,
	pub merged_modified: bool,
}

impl<T: crate::Config> ProgramCacheForTxBatch<T> {
	pub fn new(
		slot: Slot,
		environments: ProgramRuntimeEnvironments<T>,
		upcoming_environments: Option<ProgramRuntimeEnvironments<T>>,
		latest_root_epoch: Epoch,
	) -> Self {
		Self {
			entries: HashMap::new(),
			modified_entries: HashMap::new(),
			slot,
			environments,
			upcoming_environments,
			latest_root_epoch,
			hit_max_limit: false,
			loaded_missing: false,
			merged_modified: false,
		}
	}

	/// Returns the current environments depending on the given epoch
	pub fn get_environments_for_epoch(&self, epoch: Epoch) -> &ProgramRuntimeEnvironments<T> {
		if epoch != self.latest_root_epoch {
			if let Some(upcoming_environments) = self.upcoming_environments.as_ref() {
				return upcoming_environments;
			}
		}
		&self.environments
	}

	/// Refill the cache with a single entry. It's typically called during transaction loading, and
	/// transaction processing (for program management instructions).
	/// It replaces the existing entry (if any) with the provided entry. The return value contains
	/// `true` if an entry existed.
	/// The function also returns the newly inserted value.
	pub fn replenish(
		&mut self,
		key: Pubkey,
		entry: Arc<ProgramCacheEntry<T>>,
	) -> (bool, Arc<ProgramCacheEntry<T>>) {
		(self.entries.insert(key, entry.clone()).is_some(), entry)
	}

	/// Store an entry in `modified_entries` for a program modified during the
	/// transaction batch.
	pub fn store_modified_entry(&mut self, key: Pubkey, entry: Arc<ProgramCacheEntry<T>>) {
		self.modified_entries.insert(key, entry);
	}

	/// Drain the program cache's modified entries, returning the owned
	/// collection.
	pub fn drain_modified_entries(&mut self) -> HashMap<Pubkey, Arc<ProgramCacheEntry<T>>> {
		core::mem::take(&mut self.modified_entries)
	}

	pub fn find(&self, key: &Pubkey) -> Option<Arc<ProgramCacheEntry<T>>> {
		// First lookup the cache of the programs modified by the current
		// transaction. If not found, lookup the cache of the cache of the
		// programs that are loaded for the transaction batch.
		self.modified_entries.get(key).or(self.entries.get(key)).map(|entry| {
			if entry.is_implicit_delay_visibility_tombstone(self.slot) {
				// Found a program entry on the current fork, but it's not effective
				// yet. It indicates that the program has delayed visibility. Return
				// the tombstone to reflect that.
				Arc::new(ProgramCacheEntry::new_tombstone(
					entry.deployment_slot,
					entry.account_owner,
					ProgramCacheEntryType::DelayVisibility,
				))
			} else {
				entry.clone()
			}
		})
	}

	pub fn slot(&self) -> Slot {
		self.slot
	}

	pub fn set_slot_for_tests(&mut self, slot: Slot) {
		self.slot = slot;
	}

	pub fn merge(&mut self, modified_entries: &HashMap<Pubkey, Arc<ProgramCacheEntry<T>>>) {
		modified_entries.iter().for_each(|(key, entry)| {
			self.merged_modified = true;
			self.replenish(*key, entry.clone());
		})
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}
}
