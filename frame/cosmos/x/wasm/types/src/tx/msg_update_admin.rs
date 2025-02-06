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
pub struct MsgUpdateAdmin {
	pub contract: String,
	pub new_admin: String,
	pub sender: String,
}

impl TryFrom<&Any> for MsgUpdateAdmin {
	type Error = ();

	fn try_from(any: &Any) -> Result<Self, Self::Error> {
		let msg = cosmos_sdk_proto::cosmwasm::wasm::v1::MsgUpdateAdmin::decode(&mut &*any.value)
			.map_err(|_| ())?;
		Ok(Self { contract: msg.contract, new_admin: msg.new_admin, sender: msg.sender })
	}
}

impl LegacyMsg for MsgUpdateAdmin {
	const AMINO_NAME: &'static str = "wasm/MsgUpdateAdmin";
}

impl Msg for MsgUpdateAdmin {
	fn get_signers(self) -> Vec<String> {
		vec![self.sender.clone()]
	}
}
