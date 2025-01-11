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

use cosmos_sdk_proto::{prost::Message, Any};
use nostd::{string::String, vec, vec::Vec};
use pallet_cosmos_types::{coin::Coin, tx_msgs::Msg};
use pallet_cosmos_x_auth_migrations::legacytx::stdsign::LegacyMsg;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MsgInstantiateContract2 {
	pub admin: String,
	pub code_id: u64,
	pub fix_msg: bool,
	pub funds: Vec<Coin>,
	pub label: String,
	pub msg: Vec<u8>,
	pub salt: Vec<u8>,
	pub sender: String,
}

impl TryFrom<&Any> for MsgInstantiateContract2 {
	type Error = ();

	fn try_from(any: &Any) -> Result<Self, Self::Error> {
		let msg =
			cosmos_sdk_proto::cosmwasm::wasm::v1::MsgInstantiateContract2::decode(&mut &*any.value)
				.map_err(|_| ())?;
		Ok(Self {
			admin: msg.admin,
			code_id: msg.code_id,
			fix_msg: msg.fix_msg,
			funds: msg.funds.iter().map(Into::into).collect(),
			label: msg.label,
			msg: msg.msg,
			salt: msg.salt,
			sender: msg.sender,
		})
	}
}

impl LegacyMsg for MsgInstantiateContract2 {
	const AMINO_NAME: &'static str = "wasm/MsgInstantiateContract2";
}

impl Msg for MsgInstantiateContract2 {
	fn get_signers(self) -> Vec<String> {
		vec![self.sender.clone()]
	}
}
