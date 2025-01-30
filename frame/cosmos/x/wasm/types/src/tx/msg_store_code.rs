// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
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

use cosmos_sdk_proto::{prost::Message, Any};
use nostd::{string::String, vec, vec::Vec};
use pallet_cosmos_types::tx_msgs::Msg;
use pallet_cosmos_x_auth_migrations::legacytx::stdsign::LegacyMsg;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessConfig {
	pub addresses: Vec<String>,
	pub permission: i32,
}

impl From<cosmos_sdk_proto::cosmwasm::wasm::v1::AccessConfig> for AccessConfig {
	fn from(config: cosmos_sdk_proto::cosmwasm::wasm::v1::AccessConfig) -> Self {
		Self { addresses: config.addresses, permission: config.permission }
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MsgStoreCode {
	pub instantiate_permission: Option<AccessConfig>,
	pub sender: String,
	pub wasm_byte_code: Vec<u8>,
}

impl TryFrom<&Any> for MsgStoreCode {
	type Error = ();

	fn try_from(any: &Any) -> Result<Self, Self::Error> {
		let msg = cosmos_sdk_proto::cosmwasm::wasm::v1::MsgStoreCode::decode(&mut &*any.value)
			.map_err(|_| ())?;
		Ok(Self {
			instantiate_permission: msg.instantiate_permission.map(Into::into),
			sender: msg.sender,
			wasm_byte_code: msg.wasm_byte_code,
		})
	}
}

impl LegacyMsg for MsgStoreCode {
	const AMINO_NAME: &'static str = "wasm/MsgStoreCode";
}

impl Msg for MsgStoreCode {
	fn get_signers(self) -> Vec<String> {
		vec![self.sender.clone()]
	}
}
