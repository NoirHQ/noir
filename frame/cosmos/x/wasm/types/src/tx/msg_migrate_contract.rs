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
