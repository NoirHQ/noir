// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
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

use alloc::{format, vec::Vec};
use core::marker::PhantomData;
use fp_evm::{ExitError, PrecompileFailure, PrecompileHandle};
use frame_support::{
	dispatch::{DispatchClass, GetDispatchInfo, Pays, PostDispatchInfo},
	StorageHasher, Twox128,
};
use pallet_evm::{AddressMapping, FrameSystemAccountProvider, GasWeightMapping};
use pallet_evm_precompile_balances_erc20::Erc20Metadata;
use pallet_evm_precompileset_assets_erc20::AddressToAssetId;
use parity_scale_codec::{Decode, DecodeLimit, Encode};
use precompile_utils::{prelude::*, EvmResult};
use sp_runtime::traits::{Dispatchable, Get};

pub trait Config:
	pallet_assets::Config
	+ pallet_balances::Config
	+ pallet_evm::Config<AccountProvider = FrameSystemAccountProvider<Self>>
	+ Erc20Metadata
	+ AddressToAssetId<Self::AssetId>
{
	type DispatchValidator: DispatchValidate<Self::AccountId, Self::RuntimeCall>;
	type DecodeLimit: Get<u32>;
	type StorageFilter: StorageFilter;
}

pub trait DispatchValidate<AccountId, RuntimeCall> {
	fn validate_before_dispatch(
		origin: &AccountId,
		call: &RuntimeCall,
	) -> Option<PrecompileFailure>;
}

impl<AccountId, RuntimeCall> DispatchValidate<AccountId, RuntimeCall> for ()
where
	RuntimeCall: GetDispatchInfo,
{
	fn validate_before_dispatch(
		_origin: &AccountId,
		call: &RuntimeCall,
	) -> Option<PrecompileFailure> {
		let info = call.get_dispatch_info();
		if !(info.pays_fee == Pays::Yes && info.class == DispatchClass::Normal) {
			return Some(PrecompileFailure::Error {
				exit_status: ExitError::Other("invalid call".into()),
			});
		}
		None
	}
}

pub trait StorageFilter {
	fn allow(prefix: &[u8]) -> bool;
}

impl StorageFilter for () {
	fn allow(prefix: &[u8]) -> bool {
		prefix != Twox128::hash(b"Evm")
	}
}

pub struct Babel<T: Config>(PhantomData<T>);

#[precompile_utils::precompile]
impl<T> Babel<T>
where
	T: Config,
	T::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
	<T::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<T::AccountId>>,
{
	#[precompile::public("accountId(address)")]
	#[precompile::view]
	fn account_id(handle: &mut impl PrecompileHandle, owner: Address) -> EvmResult<UnboundedBytes> {
		let output = T::AddressMapping::into_account_id(owner.into()).encode();

		handle.record_db_read::<T>(output.len())?;

		Ok(output.into())
	}

	// darwinia-precompile-state-storage
	#[precompile::public("getStorage(bytes)")]
	#[precompile::view]
	fn get_storage(
		handle: &mut impl PrecompileHandle,
		key: UnboundedBytes,
	) -> EvmResult<UnboundedBytes> {
		const PALLET_PREFIX_LENGTH: usize = 16;

		let key = key.as_bytes();
		if key.len() < PALLET_PREFIX_LENGTH ||
			!T::StorageFilter::allow(&key[0..PALLET_PREFIX_LENGTH])
		{
			return Err(revert("Read restriction"));
		}

		let output = frame_support::storage::unhashed::get_raw(key).unwrap_or_default();
		// Record proof_size cost for the db content
		handle.record_db_read::<T>(output.len())?;

		Ok(output.as_slice().into())
	}

	// pallet-evm-precompile-dispatch
	#[precompile::public("dispatch(bytes)")]
	fn dispatch(handle: &mut impl PrecompileHandle, input: UnboundedBytes) -> EvmResult {
		let input: Vec<u8> = input.into();
		let target_gas = handle.gas_limit();
		let context = handle.context();

		let call = T::RuntimeCall::decode_with_depth_limit(T::DecodeLimit::get(), &mut &*input)
			.map_err(|_| PrecompileFailure::Error {
				exit_status: ExitError::Other("decode failed".into()),
			})?;
		let info = call.get_dispatch_info();

		if let Some(gas) = target_gas {
			let valid_weight = info.total_weight().ref_time() <=
				T::GasWeightMapping::gas_to_weight(gas, false).ref_time();
			if !valid_weight {
				return Err(PrecompileFailure::Error { exit_status: ExitError::OutOfGas });
			}
		}

		let origin = T::AddressMapping::into_account_id(context.caller);

		if let Some(err) = T::DispatchValidator::validate_before_dispatch(&origin, &call) {
			return Err(err);
		}

		handle.record_external_cost(
			Some(info.total_weight().ref_time()),
			Some(info.total_weight().proof_size()),
			None,
		)?;

		match call.dispatch(Some(origin).into()) {
			Ok(post_info) => {
				if post_info.pays_fee(&info) == Pays::Yes {
					let actual_weight = post_info.actual_weight.unwrap_or(info.total_weight());
					let cost = T::GasWeightMapping::weight_to_gas(actual_weight);
					handle.record_cost(cost)?;

					handle.refund_external_cost(
						Some(
							info.total_weight().ref_time().saturating_sub(actual_weight.ref_time()),
						),
						Some(
							info.total_weight()
								.proof_size()
								.saturating_sub(actual_weight.proof_size()),
						),
					);
				}

				Ok(())
			},
			Err(e) => Err(PrecompileFailure::Error {
				exit_status: ExitError::Other(
					format!("dispatch execution failed: {}", <&'static str>::from(e)).into(),
				),
			}),
		}
	}
}
