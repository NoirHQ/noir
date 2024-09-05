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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

mod apis;
mod call;
pub mod configs;
pub mod prelude;
mod primitives;
mod version;

pub use apis::*;
pub use noir_runtime_common as common;
pub use primitives::*;
pub use version::*;

use alloc::vec::Vec;
use frame_support::{parameter_types, weights::Weight};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_version::RuntimeVersion;

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(3)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(10)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(11)]
	pub type TransactionPayment = pallet_transaction_payment;

	#[runtime::pallet_index(23)]
	pub type Aura = pallet_aura;

	#[runtime::pallet_index(24)]
	pub type Grandpa = pallet_grandpa;

	//#[runtime::pallet_index(50)]
	//pub type Assets = pallet_assets;

	#[runtime::pallet_index(60)]
	pub type Ethereum = pallet_ethereum;

	#[runtime::pallet_index(61)]
	pub type Evm = pallet_evm;

	#[runtime::pallet_index(62)]
	pub type BaseFee = pallet_base_fee;

	#[runtime::pallet_index(128)]
	pub type AddressMap = pallet_multimap<Instance1>;

	#[runtime::pallet_index(255)]
	pub type Sudo = pallet_sudo;
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_evm, Evm]
		[pallet_grandpa, Grandpa]
		[pallet_sudo, Sudo]
		[pallet_timestamp, Timestamp]
	);
}
