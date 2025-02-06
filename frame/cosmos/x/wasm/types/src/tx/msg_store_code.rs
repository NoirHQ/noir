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
