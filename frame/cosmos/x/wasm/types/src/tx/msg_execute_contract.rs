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
pub struct MsgExecuteContract {
	pub contract: String,
	pub funds: Vec<Coin>,
	pub msg: Vec<u8>,
	pub sender: String,
}

impl TryFrom<&Any> for MsgExecuteContract {
	type Error = ();

	fn try_from(any: &Any) -> Result<Self, Self::Error> {
		let msg =
			cosmos_sdk_proto::cosmwasm::wasm::v1::MsgExecuteContract::decode(&mut &*any.value)
				.map_err(|_| ())?;
		Ok(Self {
			contract: msg.contract,
			funds: msg.funds.iter().map(Into::into).collect(),
			msg: msg.msg,
			sender: msg.sender,
		})
	}
}

impl LegacyMsg for MsgExecuteContract {
	const AMINO_NAME: &'static str = "wasm/MsgExecuteContract";
}

impl Msg for MsgExecuteContract {
	fn get_signers(self) -> Vec<String> {
		vec![self.sender.clone()]
	}
}
