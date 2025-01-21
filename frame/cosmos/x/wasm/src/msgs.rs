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

use crate::error::handle_vm_error;
use alloc::{string::ToString, vec, vec::Vec};
use core::{marker::PhantomData, str::FromStr};
use cosmos_sdk_proto::{
	cosmos::base::v1beta1::Coin,
	cosmwasm::wasm::v1::{
		MsgExecuteContract, MsgInstantiateContract2, MsgMigrateContract, MsgStoreCode,
		MsgUpdateAdmin,
	},
	prost::Message,
	Any,
};
use frame_support::ensure;
use libflate::gzip::Decoder;
use nostd::io::Read;
use pallet_cosmos::AddressMapping;
use pallet_cosmos_types::{
	address::acc_address_from_bech32,
	context,
	errors::{CosmosError, RootError},
	events::{traits::EventManager, CosmosEvent, EventAttribute},
	gas::traits::GasMeter,
	msgservice::traits::MsgHandler,
};
use pallet_cosmos_x_wasm_types::{
	errors::WasmError,
	events::{
		ATTRIBUTE_KEY_CHECKSUM, ATTRIBUTE_KEY_CODE_ID, ATTRIBUTE_KEY_CONTRACT_ADDR,
		ATTRIBUTE_KEY_NEW_ADMIN, EVENT_TYPE_EXECUTE, EVENT_TYPE_INSTANTIATE, EVENT_TYPE_MIGRATE,
		EVENT_TYPE_STORE_CODE, EVENT_TYPE_UPDATE_CONTRACT_ADMIN,
	},
};
use pallet_cosmwasm::{
	runtimes::vm::InitialStorageMutability,
	types::{
		CodeIdentifier, ContractCodeOf, ContractLabelOf, ContractMessageOf, ContractSaltOf, FundsOf,
	},
};
use sp_core::H160;
use sp_runtime::{
	traits::{Convert, TryConvert, TryConvertBack},
	SaturatedConversion,
};

pub struct MsgStoreCodeHandler<T>(PhantomData<T>);

impl<T> Default for MsgStoreCodeHandler<T> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<T, Context> MsgHandler<Context> for MsgStoreCodeHandler<T>
where
	T: pallet_cosmos::Config + pallet_cosmwasm::Config,
	Context: context::traits::Context,
{
	fn handle(&self, ctx: &mut Context, msg: &Any) -> Result<(), CosmosError> {
		let MsgStoreCode { sender, wasm_byte_code, instantiate_permission: _ } =
			MsgStoreCode::decode(&mut &*msg.value).map_err(|_| RootError::TxDecodeError)?;

		ensure!(!sender.is_empty(), WasmError::Empty);
		let (_hrp, address_raw) =
			acc_address_from_bech32(&sender).map_err(|_| RootError::InvalidAddress)?;
		ensure!(address_raw.len() == 20, RootError::InvalidAddress);
		let who = T::AddressMapping::into_account_id(H160::from_slice(&address_raw));

		ctx.gas_meter()
			.consume_gas(wasm_byte_code.len() as u64, "")
			.map_err(|_| RootError::OutOfGas)?;

		let mut decoder = Decoder::new(&wasm_byte_code[..]).map_err(|_| WasmError::CreateFailed)?;
		let mut decoded_code = Vec::new();
		decoder.read_to_end(&mut decoded_code).map_err(|_| WasmError::CreateFailed)?;
		let code: ContractCodeOf<T> =
			decoded_code.try_into().map_err(|_| WasmError::CreateFailed)?;

		let (code_hash, code_id) = pallet_cosmwasm::Pallet::<T>::do_upload(&who, code)
			.map_err(|_| WasmError::CreateFailed)?;

		// TODO: Same events emitted pallet_cosmos and pallet_cosmwasm
		let msg_event = CosmosEvent {
			r#type: EVENT_TYPE_STORE_CODE.into(),
			attributes: vec![
				EventAttribute {
					key: ATTRIBUTE_KEY_CODE_ID.into(),
					value: code_id.to_string().into(),
				},
				EventAttribute {
					key: ATTRIBUTE_KEY_CHECKSUM.into(),
					value: const_hex::encode(code_hash.0).into(),
				},
			],
		};
		ctx.event_manager().emit_event(msg_event);

		Ok(())
	}
}

pub struct MsgInstantiateContract2Handler<T>(PhantomData<T>);

impl<T> Default for MsgInstantiateContract2Handler<T> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<T, Context> MsgHandler<Context> for MsgInstantiateContract2Handler<T>
where
	T: pallet_cosmos::Config + pallet_cosmwasm::Config,
	Context: context::traits::Context,
{
	fn handle(&self, ctx: &mut Context, msg: &Any) -> Result<(), CosmosError> {
		let MsgInstantiateContract2 { sender, admin, code_id, label, msg, funds, salt, fix_msg: _ } =
			MsgInstantiateContract2::decode(&mut &*msg.value)
				.map_err(|_| RootError::TxDecodeError)?;

		ensure!(!sender.is_empty(), WasmError::Empty);
		let (_hrp, address_raw) =
			acc_address_from_bech32(&sender).map_err(|_| RootError::InvalidAddress)?;
		ensure!(address_raw.len() == 20, RootError::InvalidAddress);
		let who = T::AddressMapping::into_account_id(H160::from_slice(&address_raw));

		let gas = ctx.gas_meter().gas_remaining();
		let mut shared = pallet_cosmwasm::Pallet::<T>::do_create_vm_shared(
			gas,
			InitialStorageMutability::ReadWrite,
		);
		let code_identifier = CodeIdentifier::CodeId(code_id);

		let admin_account = if !admin.is_empty() {
			let admin_account =
				T::AccountToAddr::try_convert(admin).map_err(|_| RootError::InvalidAddress)?;
			Some(admin_account)
		} else {
			None
		};

		let salt: ContractSaltOf<T> = salt.try_into().map_err(|_| RootError::TxDecodeError)?;
		let label: ContractLabelOf<T> =
			label.as_bytes().to_vec().try_into().map_err(|_| RootError::TxDecodeError)?;
		let funds = convert_funds::<T>(&funds)?;
		let message: ContractMessageOf<T> = msg.try_into().map_err(|_| RootError::TxDecodeError)?;

		let contract_account = pallet_cosmwasm::Pallet::<T>::do_instantiate(
			&mut shared,
			who,
			code_identifier,
			salt,
			admin_account,
			label,
			funds,
			message,
		)
		.map_err(|e| handle_vm_error(e, WasmError::InstantiateFailed))?;
		ctx.gas_meter()
			.consume_gas(gas.saturating_sub(shared.gas.remaining()), "")
			.map_err(|_| RootError::OutOfGas)?;

		let contract_address = T::AccountToAddr::convert(contract_account);

		// TODO: Same events emitted pallet_cosmos and pallet_cosmwasm
		let msg_event = CosmosEvent {
			r#type: EVENT_TYPE_INSTANTIATE.into(),
			attributes: vec![
				EventAttribute {
					key: ATTRIBUTE_KEY_CONTRACT_ADDR.into(),
					value: contract_address.into(),
				},
				EventAttribute {
					key: ATTRIBUTE_KEY_CODE_ID.into(),
					value: code_id.to_string().into(),
				},
			],
		};
		ctx.event_manager().emit_event(msg_event);

		Ok(())
	}
}

pub struct MsgExecuteContractHandler<T>(PhantomData<T>);

impl<T> Default for MsgExecuteContractHandler<T> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<T, Context> MsgHandler<Context> for MsgExecuteContractHandler<T>
where
	T: pallet_cosmos::Config + pallet_cosmwasm::Config,
	Context: context::traits::Context,
{
	fn handle(&self, ctx: &mut Context, msg: &Any) -> Result<(), CosmosError> {
		let MsgExecuteContract { sender, contract, msg, funds } =
			MsgExecuteContract::decode(&mut &*msg.value).map_err(|_| RootError::TxDecodeError)?;

		ensure!(!sender.is_empty(), WasmError::Empty);
		let (_hrp, address_raw) =
			acc_address_from_bech32(&sender).map_err(|_| RootError::InvalidAddress)?;
		ensure!(address_raw.len() == 20, RootError::InvalidAddress);
		let who = T::AddressMapping::into_account_id(H160::from_slice(&address_raw));

		let gas = ctx.gas_meter().gas_remaining();
		let mut shared = pallet_cosmwasm::Pallet::<T>::do_create_vm_shared(
			gas,
			InitialStorageMutability::ReadWrite,
		);

		let contract_account = T::AccountToAddr::try_convert(contract.clone())
			.map_err(|_| RootError::TxDecodeError)?;
		let funds: FundsOf<T> = convert_funds::<T>(&funds)?;
		let message: ContractMessageOf<T> = msg.try_into().map_err(|_| RootError::TxDecodeError)?;

		pallet_cosmwasm::Pallet::<T>::do_execute(
			&mut shared,
			who,
			contract_account,
			funds,
			message,
		)
		.map_err(|e| handle_vm_error(e, WasmError::ExecuteFailed))?;
		ctx.gas_meter()
			.consume_gas(gas.saturating_sub(shared.gas.remaining()), "")
			.map_err(|_| RootError::OutOfGas)?;

		// TODO: Same events emitted pallet_cosmos and pallet_cosmwasm
		let msg_event = CosmosEvent {
			r#type: EVENT_TYPE_EXECUTE.into(),
			attributes: vec![EventAttribute {
				key: ATTRIBUTE_KEY_CONTRACT_ADDR.into(),
				value: contract.into(),
			}],
		};
		ctx.event_manager().emit_event(msg_event);

		Ok(())
	}
}

fn convert_funds<T: pallet_cosmwasm::Config>(coins: &[Coin]) -> Result<FundsOf<T>, CosmosError> {
	let mut funds = FundsOf::<T>::default();
	for coin in coins.iter() {
		let asset_id = T::AssetToDenom::try_convert_back(coin.denom.clone())
			.map_err(|_| RootError::TxDecodeError)?;
		let amount = u128::from_str(&coin.amount).map_err(|_| RootError::TxDecodeError)?;

		funds
			.try_insert(asset_id, (amount.saturated_into(), true))
			.map_err(|_| RootError::TxDecodeError)?;
	}

	Ok(funds)
}

pub struct MsgMigrateContractHandler<T>(PhantomData<T>);

impl<T> Default for MsgMigrateContractHandler<T> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<T, Context> MsgHandler<Context> for MsgMigrateContractHandler<T>
where
	T: pallet_cosmos::Config + pallet_cosmwasm::Config,
	Context: context::traits::Context,
{
	fn handle(&self, ctx: &mut Context, msg: &Any) -> Result<(), CosmosError> {
		let MsgMigrateContract { sender, contract, code_id, msg } =
			MsgMigrateContract::decode(&mut &*msg.value).map_err(|_| RootError::TxDecodeError)?;

		ensure!(!sender.is_empty(), WasmError::Empty);
		let (_hrp, address_raw) =
			acc_address_from_bech32(&sender).map_err(|_| RootError::InvalidAddress)?;
		ensure!(address_raw.len() == 20, RootError::InvalidAddress);
		let who = T::AddressMapping::into_account_id(H160::from_slice(&address_raw));

		let gas = ctx.gas_meter().gas_remaining();
		let mut shared = pallet_cosmwasm::Pallet::<T>::do_create_vm_shared(
			gas,
			InitialStorageMutability::ReadWrite,
		);

		let contract_account = T::AccountToAddr::try_convert(contract.clone())
			.map_err(|_| RootError::TxDecodeError)?;
		let new_code_identifier = CodeIdentifier::CodeId(code_id);
		let message: ContractMessageOf<T> = msg.try_into().map_err(|_| RootError::TxDecodeError)?;

		pallet_cosmwasm::Pallet::<T>::do_migrate(
			&mut shared,
			who,
			contract_account,
			new_code_identifier,
			message,
		)
		.map_err(|e| handle_vm_error(e, WasmError::MigrationFailed))?;
		ctx.gas_meter()
			.consume_gas(gas.saturating_sub(shared.gas.remaining()), "")
			.map_err(|_| RootError::OutOfGas)?;

		// TODO: Same events emitted pallet_cosmos and pallet_cosmwasm
		let msg_event = CosmosEvent {
			r#type: EVENT_TYPE_MIGRATE.into(),
			attributes: vec![
				EventAttribute {
					key: ATTRIBUTE_KEY_CODE_ID.into(),
					value: code_id.to_string().into(),
				},
				EventAttribute { key: ATTRIBUTE_KEY_CONTRACT_ADDR.into(), value: contract.into() },
			],
		};
		ctx.event_manager().emit_event(msg_event);

		Ok(())
	}
}

pub struct MsgUpdateAdminHandler<T>(PhantomData<T>);

impl<T> Default for MsgUpdateAdminHandler<T> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<T, Context> MsgHandler<Context> for MsgUpdateAdminHandler<T>
where
	T: pallet_cosmos::Config + pallet_cosmwasm::Config,
	Context: context::traits::Context,
{
	fn handle(&self, ctx: &mut Context, msg: &Any) -> Result<(), CosmosError> {
		let MsgUpdateAdmin { sender, new_admin, contract } =
			MsgUpdateAdmin::decode(&mut &*msg.value).map_err(|_| RootError::TxDecodeError)?;

		ensure!(!sender.is_empty(), WasmError::Empty);
		let (_hrp, address_raw) =
			acc_address_from_bech32(&sender).map_err(|_| RootError::InvalidAddress)?;
		ensure!(address_raw.len() == 20, RootError::InvalidAddress);
		let who = T::AddressMapping::into_account_id(H160::from_slice(&address_raw));

		let gas = ctx.gas_meter().gas_remaining();
		let mut shared = pallet_cosmwasm::Pallet::<T>::do_create_vm_shared(
			gas,
			InitialStorageMutability::ReadWrite,
		);

		let new_admin_account = if !new_admin.is_empty() {
			let new_admin_account = T::AccountToAddr::try_convert(new_admin.clone())
				.map_err(|_| RootError::InvalidAddress)?;
			Some(new_admin_account)
		} else {
			None
		};
		let contract_account = T::AccountToAddr::try_convert(contract.clone())
			.map_err(|_| RootError::TxDecodeError)?;

		pallet_cosmwasm::Pallet::<T>::do_update_admin(
			&mut shared,
			who,
			contract_account,
			new_admin_account,
		)
		.map_err(|e| handle_vm_error(e, RootError::Unauthorized))?;
		ctx.gas_meter()
			.consume_gas(gas.saturating_sub(shared.gas.remaining()), "")
			.map_err(|_| RootError::OutOfGas)?;

		// TODO: Same events emitted pallet_cosmos and pallet_cosmwasm
		let msg_event = CosmosEvent {
			r#type: EVENT_TYPE_UPDATE_CONTRACT_ADMIN.into(),
			attributes: vec![
				EventAttribute { key: ATTRIBUTE_KEY_CONTRACT_ADDR.into(), value: contract.into() },
				EventAttribute { key: ATTRIBUTE_KEY_NEW_ADMIN.into(), value: new_admin.into() },
			],
		};
		ctx.event_manager().emit_event(msg_event);

		Ok(())
	}
}
