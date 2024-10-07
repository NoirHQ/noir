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

pub mod traits;

use alloc::{string::String, vec::Vec};
use cosmos_sdk_proto::{
	cosmos::{bank, tx::v1beta1::Tx},
	cosmwasm::wasm,
};
use pallet_cosmos_types::{
	any_match,
	tx_msgs::{FeeTx, Msg},
};
use pallet_cosmos_x_bank_types::msgs::msg_send::MsgSend;
use pallet_cosmos_x_wasm_types::tx::{
	msg_execute_contract::MsgExecuteContract, msg_instantiate_contract2::MsgInstantiateContract2,
	msg_migrate_contract::MsgMigrateContract, msg_store_code::MsgStoreCode,
	msg_update_admin::MsgUpdateAdmin,
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SigVerifiableTxError {
	EmptyAuthInfo,
	EmptyFee,
	EmptySigners,
	EmptyTxBody,
	InvalidMsg,
}

pub struct SigVerifiableTx;
impl traits::SigVerifiableTx for SigVerifiableTx {
	fn get_signers(tx: &Tx) -> Result<Vec<String>, SigVerifiableTxError> {
		let mut signers = Vec::<String>::new();

		let body = tx.body.as_ref().ok_or(SigVerifiableTxError::EmptyTxBody)?;
		for msg in body.messages.iter() {
			let msg_signers = any_match!(
				msg, {
					bank::v1beta1::MsgSend => MsgSend::try_from(msg).map(Msg::get_signers).map_err(|_| SigVerifiableTxError::InvalidMsg),
					wasm::v1::MsgStoreCode => MsgStoreCode::try_from(msg).map(Msg::get_signers).map_err(|_| SigVerifiableTxError::InvalidMsg),
					wasm::v1::MsgInstantiateContract2 => MsgInstantiateContract2::try_from(msg).map(Msg::get_signers).map_err(|_| SigVerifiableTxError::InvalidMsg),
					wasm::v1::MsgExecuteContract => MsgExecuteContract::try_from(msg).map(Msg::get_signers).map_err(|_| SigVerifiableTxError::InvalidMsg),
					wasm::v1::MsgMigrateContract => MsgMigrateContract::try_from(msg).map(Msg::get_signers).map_err(|_| SigVerifiableTxError::InvalidMsg),
					wasm::v1::MsgUpdateAdmin => MsgUpdateAdmin::try_from(msg).map(Msg::get_signers).map_err(|_| SigVerifiableTxError::InvalidMsg),
				},
				Err(SigVerifiableTxError::InvalidMsg)
			)?;

			for msg_signer in msg_signers.iter() {
				if !signers.contains(msg_signer) {
					signers.push(msg_signer.clone());
				}
			}
		}

		let fee_payer = tx.fee_payer().ok_or(SigVerifiableTxError::EmptyFee)?;
		if !fee_payer.is_empty() && !signers.contains(&fee_payer) {
			signers.push(fee_payer.clone());
		}

		Ok(signers)
	}

	fn fee_payer(tx: &Tx) -> Result<String, SigVerifiableTxError> {
		let fee_payer = tx.fee_payer().ok_or(SigVerifiableTxError::EmptyFee)?;

		if !fee_payer.is_empty() {
			Ok(fee_payer)
		} else {
			Self::get_signers(tx)?
				.first()
				.ok_or(SigVerifiableTxError::EmptySigners)
				.cloned()
		}
	}

	fn sequence(tx: &Tx) -> Result<u64, SigVerifiableTxError> {
		let auth_info = tx.auth_info.as_ref().ok_or(SigVerifiableTxError::EmptyAuthInfo)?;
		let fee = auth_info.fee.as_ref().ok_or(SigVerifiableTxError::EmptyFee)?;

		let sequence = if fee.payer.is_empty() {
			auth_info.signer_infos.first()
		} else {
			auth_info.signer_infos.last()
		}
		.ok_or(SigVerifiableTxError::EmptySigners)?
		.sequence;

		Ok(sequence)
	}
}
