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
use pallet_cosmos_types::{coin::Coin, tx_msgs::Msg};
use pallet_cosmos_x_auth_migrations::legacytx::stdsign::LegacyMsg;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MsgSend {
	pub amount: Vec<Coin>,
	pub from_address: String,
	pub to_address: String,
}

impl TryFrom<&Any> for MsgSend {
	type Error = ();

	fn try_from(any: &Any) -> Result<Self, Self::Error> {
		let msg = cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend::decode(&mut &*any.value)
			.map_err(|_| ())?;

		Ok(Self {
			amount: msg.amount.iter().map(Into::into).collect(),
			from_address: msg.from_address,
			to_address: msg.to_address,
		})
	}
}

impl Msg for MsgSend {
	fn get_signers(self) -> Vec<String> {
		vec![self.from_address.clone()]
	}
}

impl LegacyMsg for MsgSend {
	const AMINO_NAME: &'static str = "cosmos-sdk/MsgSend";
}
