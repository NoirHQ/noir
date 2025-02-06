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

pub mod traits;

use cosmos_sdk_proto::{
	cosmos::{bank, tx::v1beta1::Tx},
	cosmwasm::wasm,
};
use nostd::{string::String, vec::Vec};
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
