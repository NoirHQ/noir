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

use alloc::{string::String, vec, vec::Vec};
use cosmos_sdk_proto::{prost::Message, Any};
use pallet_cosmos_types::tx_msgs::Msg;
use pallet_cosmos_x_auth_migrations::legacytx::stdsign::LegacyMsg;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MsgMigrateContract {
	pub code_id: u64,
	pub contract: String,
	pub msg: Vec<u8>,
	pub sender: String,
}

impl TryFrom<&Any> for MsgMigrateContract {
	type Error = ();

	fn try_from(any: &Any) -> Result<Self, Self::Error> {
		let msg =
			cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContract::decode(&mut &*any.value)
				.map_err(|_| ())?;
		Ok(Self { code_id: msg.code_id, contract: msg.contract, msg: msg.msg, sender: msg.sender })
	}
}

impl LegacyMsg for MsgMigrateContract {
	const AMINO_NAME: &'static str = "wasm/MsgMigrateContract";
}

impl Msg for MsgMigrateContract {
	fn get_signers(self) -> Vec<String> {
		vec![self.sender.clone()]
	}
}
