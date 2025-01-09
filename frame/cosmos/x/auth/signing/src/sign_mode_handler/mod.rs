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

use cosmos_sdk_proto::{
	cosmos::{
		bank,
		tx::{
			signing::v1beta1::SignMode,
			v1beta1::{
				mode_info::{Single, Sum},
				ModeInfo, SignDoc, Tx, TxRaw,
			},
		},
	},
	cosmwasm::wasm,
	traits::Message,
	Any,
};
use nostd::{
	string::{String, ToString},
	vec::Vec,
};
use pallet_cosmos_types::{any_match, tx_msgs::FeeTx};
use pallet_cosmos_x_auth_migrations::legacytx::stdsign::{LegacyMsg, StdSignDoc};
use pallet_cosmos_x_bank_types::msgs::msg_send::MsgSend;
use pallet_cosmos_x_wasm_types::tx::{
	msg_execute_contract::MsgExecuteContract, msg_instantiate_contract2::MsgInstantiateContract2,
	msg_migrate_contract::MsgMigrateContract, msg_store_code::MsgStoreCode,
	msg_update_admin::MsgUpdateAdmin,
};
use serde_json::Value;

#[derive(Clone)]
pub struct SignerData {
	pub address: String,
	pub chain_id: String,
	pub account_number: u64,
	pub sequence: u64,
	pub pub_key: Any,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SignModeHandlerError {
	EmptyTxBody,
	EmptyFee,
	EmptyModeInfo,
	DecodeTxError,
	InvalidMsg,
	SerializeError,
	UnsupportedMode,
}

const SIGN_MODE_DIRECT: i32 = SignMode::Direct as i32;
const SIGN_MODE_LEGACY_AMINO_JSON: i32 = SignMode::LegacyAminoJson as i32;

pub struct SignModeHandler;
impl traits::SignModeHandler for SignModeHandler {
	fn get_sign_bytes(
		mode: &ModeInfo,
		data: &SignerData,
		tx: &Tx,
	) -> Result<Vec<u8>, SignModeHandlerError> {
		let sum = mode.sum.as_ref().ok_or(SignModeHandlerError::EmptyModeInfo)?;
		let sign_bytes = match sum {
			Sum::Single(Single { mode }) => match *mode {
				SIGN_MODE_DIRECT => {
					let tx_raw = TxRaw::decode(&mut &*tx.encode_to_vec())
						.map_err(|_| SignModeHandlerError::DecodeTxError)?;
					SignDoc {
						body_bytes: tx_raw.body_bytes,
						auth_info_bytes: tx_raw.auth_info_bytes,
						chain_id: data.chain_id.clone(),
						account_number: data.account_number,
					}
					.encode_to_vec()
				},
				SIGN_MODE_LEGACY_AMINO_JSON => {
					let body = tx.body.as_ref().ok_or(SignModeHandlerError::EmptyTxBody)?;
					let mut msgs = Vec::<Value>::new();
					for msg in body.messages.iter() {
						let legacy_msg = any_match!(
							msg, {
								bank::v1beta1::MsgSend => MsgSend::try_from(msg).map(LegacyMsg::get_sign_bytes).map_err(|_| SignModeHandlerError::InvalidMsg),
								wasm::v1::MsgStoreCode => MsgStoreCode::try_from(msg).map(LegacyMsg::get_sign_bytes).map_err(|_| SignModeHandlerError::InvalidMsg),
								wasm::v1::MsgInstantiateContract2 => MsgInstantiateContract2::try_from(msg).map(LegacyMsg::get_sign_bytes).map_err(|_| SignModeHandlerError::InvalidMsg),
								wasm::v1::MsgExecuteContract => MsgExecuteContract::try_from(msg).map(LegacyMsg::get_sign_bytes).map_err(|_| SignModeHandlerError::InvalidMsg),
								wasm::v1::MsgMigrateContract => MsgMigrateContract::try_from(msg).map(LegacyMsg::get_sign_bytes).map_err(|_| SignModeHandlerError::InvalidMsg),
								wasm::v1::MsgUpdateAdmin => MsgUpdateAdmin::try_from(msg).map(LegacyMsg::get_sign_bytes).map_err(|_| SignModeHandlerError::InvalidMsg),
							},
							Err(SignModeHandlerError::InvalidMsg))?;

						msgs.push(legacy_msg);
					}
					let fee = tx.fee().ok_or(SignModeHandlerError::EmptyFee)?;
					let sign_doc = StdSignDoc {
						account_number: data.account_number.to_string(),
						chain_id: data.chain_id.clone(),
						fee: fee.into(),
						memo: body.memo.clone(),
						msgs,
						sequence: data.sequence.to_string(),
					};

					serde_json::to_vec(&sign_doc)
						.map_err(|_| SignModeHandlerError::SerializeError)?
				},
				_ => return Err(SignModeHandlerError::UnsupportedMode),
			},
			_ => return Err(SignModeHandlerError::UnsupportedMode),
		};

		Ok(sign_bytes)
	}
}

#[cfg(test)]
mod tests {
	use crate::sign_mode_handler::{traits::SignModeHandler as _, SignModeHandler, SignerData};
	use base64::{prelude::BASE64_STANDARD, Engine};
	use cosmos_sdk_proto::{
		cosmos::tx::v1beta1::{
			mode_info::{Single, Sum},
			ModeInfo, Tx,
		},
		prost::Message,
	};
	use sp_core::sha2_256;

	#[test]
	fn get_sign_bytes_test() {
		let tx_raw = "CpMBCpABChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEnAKLWNvc21vczFxZDY5bnV3ajk1Z3RhNGFramd5eHRqOXVqbXo0dzhlZG1xeXNxdxItY29zbW9zMWdtajJleGFnMDN0dGdhZnBya2RjM3Q4ODBncm1hOW53ZWZjZDJ3GhAKBXVhdG9tEgcxMDAwMDAwEnEKTgpGCh8vY29zbW9zLmNyeXB0by5zZWNwMjU2azEuUHViS2V5EiMKIQIKEJE0H+VmS/oXgtXgR3lokGjJFrBMs2XsMVN1VoTZoRIECgIIARIfChUKBXVhdG9tEgw4ODY4ODAwMDAwMDAQgMDxxZSVFBpA9+DRmMYoIcxYF8jpNfUjMIMB4pgZ9diC8ySbnhc6YU84AA3b/0RsCr+nx9AZ27FwcrKJM/yBh8lz+/A9BFn3bg==";

		let tx_raw = BASE64_STANDARD.decode(tx_raw).unwrap();
		let tx = Tx::decode(&mut &*tx_raw).unwrap();

		let public_key = tx
			.auth_info
			.as_ref()
			.unwrap()
			.signer_infos
			.first()
			.unwrap()
			.public_key
			.as_ref()
			.unwrap();

		let mode = ModeInfo { sum: Some(Sum::Single(Single { mode: 1 })) };
		let data = SignerData {
			address: "cosmos1qd69nuwj95gta4akjgyxtj9ujmz4w8edmqysqw".to_string(),
			chain_id: "theta-testnet-001".to_string(),
			account_number: 754989,
			sequence: 0,
			pub_key: public_key.clone(),
		};
		let expected_hash = sha2_256(&SignModeHandler::get_sign_bytes(&mode, &data, &tx).unwrap());

		let sign_doc_raw =
		"CpMBCpABChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEnAKLWNvc21vczFxZDY5bnV3ajk1Z3RhNGFramd5eHRqOXVqbXo0dzhlZG1xeXNxdxItY29zbW9zMWdtajJleGFnMDN0dGdhZnBya2RjM3Q4ODBncm1hOW53ZWZjZDJ3GhAKBXVhdG9tEgcxMDAwMDAwEnEKTgpGCh8vY29zbW9zLmNyeXB0by5zZWNwMjU2azEuUHViS2V5EiMKIQIKEJE0H+VmS/oXgtXgR3lokGjJFrBMs2XsMVN1VoTZoRIECgIIARIfChUKBXVhdG9tEgw4ODY4ODAwMDAwMDAQgMDxxZSVFBoRdGhldGEtdGVzdG5ldC0wMDEgrYou";
		let hash = sha2_256(&BASE64_STANDARD.decode(sign_doc_raw).unwrap());

		assert_eq!(expected_hash, hash);
	}

	#[test]
	fn get_std_sign_bytes_test() {
		let tx_raw =  "CpoBCpcBChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEncKLWNvc21vczFxZDY5bnV3ajk1Z3RhNGFramd5eHRqOXVqbXo0dzhlZG1xeXNxdxItY29zbW9zMW41amd4NjR6dzM4c3M3Nm16dXU0dWM3amV5cXcydmZqazYwZmR6GhcKBGFjZHQSDzEwMDAwMDAwMDAwMDAwMBJsCk4KRgofL2Nvc21vcy5jcnlwdG8uc2VjcDI1NmsxLlB1YktleRIjCiECChCRNB/lZkv6F4LV4Ed5aJBoyRawTLNl7DFTdVaE2aESBAoCCH8SGgoSCgRhY2R0EgoxMDQwMDAwMDAwEIDa8esEGkBgXIiPoBpecG7QpKDJPaztFogqvmxjDHF5ORfWBrOoSzf0+AAmch1CXrG4OmiKL0y8v9ITx0QzWYUc7ueXcdIm";
		let tx_raw = BASE64_STANDARD.decode(tx_raw).unwrap();
		let tx = Tx::decode(&mut &*tx_raw).unwrap();

		let public_key = tx
			.auth_info
			.as_ref()
			.unwrap()
			.signer_infos
			.first()
			.unwrap()
			.public_key
			.as_ref()
			.unwrap();

		let mode = ModeInfo { sum: Some(Sum::Single(Single { mode: 127 })) };
		let data = SignerData {
			address: "cosmos1qd69nuwj95gta4akjgyxtj9ujmz4w8edmqysqw".to_string(),
			chain_id: "dev".to_string(),
			account_number: 0,
			sequence: 0,
			pub_key: public_key.clone(),
		};
		let hash = sha2_256(&SignModeHandler::get_sign_bytes(&mode, &data, &tx).unwrap());
		let hash = hex::encode(hash);

		assert_eq!(hash, "714d4bdfdbd0bd630ebdf93b1f6eba7d3c752e92bbab6c9d3d9c93e1777348bb");
	}
}
