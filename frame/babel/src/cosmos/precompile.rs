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

use alloc::vec::Vec;
use core::marker::PhantomData;
use cosmwasm_std::{Binary, ContractResult, Response};
use cosmwasm_vm::{
	executor::QueryResponse,
	vm::{VMBase, VmErrorOf},
};
use cosmwasm_vm_wasmi::OwnedWasmiVM;
use frame_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	ensure, PalletId,
};
use pallet_cosmos::AddressMapping;
use pallet_cosmos_types::address::{acc_address_from_bech32, AUTH_ADDRESS_LEN};
use pallet_cosmwasm::{
	pallet_hook::PalletHook,
	runtimes::vm::{CosmwasmVM, CosmwasmVMError},
	types::{AccountIdOf, ContractLabelOf, ContractTrieIdOf, EntryPoint, PalletContractCodeInfo},
};
use parity_scale_codec::{Decode, DecodeLimit};
use serde::{Deserialize, Serialize};
use sp_core::H160;
use sp_runtime::traits::{AccountIdConversion, Convert, Dispatchable};

const ID: PalletId = PalletId(*b"dispatch");
const DECODE_LIMIT: u32 = 8;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
	Dispatch { input: Binary },
}

pub struct Precompiles<T>(PhantomData<T>);
impl<T> PalletHook<T> for Precompiles<T>
where
	T: pallet_cosmwasm::Config,
	T: pallet_cosmos::Config,
	T::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
	<T::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<T::AccountId>>,
{
	fn info(
		contract_address: &AccountIdOf<T>,
	) -> Option<PalletContractCodeInfo<AccountIdOf<T>, ContractLabelOf<T>, ContractTrieIdOf<T>>> {
		let dispatch = AccountIdConversion::<T::AccountId>::into_account_truncating(&ID);

		match contract_address {
			address if address == &dispatch => Some(PalletContractCodeInfo::new(
				dispatch,
				false,
				ID.0.to_vec().try_into().unwrap_or_default(),
			)),
			_ => None,
		}
	}

	fn execute<'a>(
		vm: &mut OwnedWasmiVM<CosmwasmVM<'a, T>>,
		_entrypoint: EntryPoint,
		message: &[u8],
	) -> Result<
		ContractResult<Response<<OwnedWasmiVM<CosmwasmVM<'a, T>> as VMBase>::MessageCustom>>,
		VmErrorOf<OwnedWasmiVM<CosmwasmVM<'a, T>>>,
	> {
		let contract_address = vm.0.data().contract_address.clone().into_inner();
		let dispatch = AccountIdConversion::<T::AccountId>::into_account_truncating(&ID);
		match contract_address {
			address if address == dispatch => {
				if let Ok(ExecuteMsg::Dispatch { input }) = serde_json_wasm::from_slice(message) {
					let call = T::RuntimeCall::decode_with_depth_limit(DECODE_LIMIT, &mut &*input)
						.map_err(|_| CosmwasmVMError::ExecuteDeserialize)?;
					let weight = call.get_dispatch_info().weight;
					vm.0.data_mut()
						.charge_raw(T::WeightToGas::convert(weight))
						.map_err(|_| CosmwasmVMError::OutOfGas)?;

					let sender = vm.0.data().cosmwasm_message_info.sender.clone().into_string();
					let (_hrp, address_raw) = acc_address_from_bech32(&sender)
						.map_err(|_| CosmwasmVMError::AccountConvert)?;
					ensure!(address_raw.len() == AUTH_ADDRESS_LEN, CosmwasmVMError::AccountConvert);

					let origin = T::AddressMapping::into_account_id(H160::from_slice(&address_raw));

					call.dispatch(Some(origin).into())
						.map_err(|e| CosmwasmVMError::SubstrateDispatch(e.error))?;

					Ok(ContractResult::Ok(Response::new()))
				} else {
					Err(CosmwasmVMError::ExecuteDeserialize)
				}
			},
			_ => Err(CosmwasmVMError::ContractNotFound),
		}
	}

	fn run<'a>(
		_vm: &mut OwnedWasmiVM<CosmwasmVM<'a, T>>,
		_entrypoint: EntryPoint,
		_message: &[u8],
	) -> Result<Vec<u8>, VmErrorOf<OwnedWasmiVM<CosmwasmVM<'a, T>>>> {
		Err(CosmwasmVMError::ContractNotFound)
	}

	fn query<'a>(
		_vm: &mut OwnedWasmiVM<pallet_cosmwasm::runtimes::vm::CosmwasmVM<'a, T>>,
		_message: &[u8],
	) -> Result<
		ContractResult<QueryResponse>,
		VmErrorOf<OwnedWasmiVM<pallet_cosmwasm::runtimes::vm::CosmwasmVM<'a, T>>>,
	> {
		Err(CosmwasmVMError::ContractNotFound)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn deserialize_msg_test() {
		let message = r#"{ "dispatch": { "input" : "CgMAkLWrIFxpdMnqhBvmiIZGM9ycqKNXhD7qzyMUZJll/iIPAADBb/KGIw==" } }"#;
		let ExecuteMsg::Dispatch { input } =
			serde_json_wasm::from_slice(message.as_bytes()).unwrap();

		assert_eq!(input, const_hex::decode("0a030090b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe220f0000c16ff28623").unwrap());
	}
}
