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

use nostd::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

pub const EVENT_TYPE_TX: &str = "tx";

pub const ATTRIBUTE_KEY_FEE: &str = "fee";
pub const ATTRIBUTE_KEY_FEE_PAYER: &str = "fee_payer";

pub const EVENT_TYPE_MESSAGE: &str = "message";

pub const ATTRIBUTE_KEY_SENDER: &str = "sender";
pub const ATTRIBUTE_KEY_AMOUNT: &str = "amount";

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub struct CosmosEvent {
	#[serde(rename = "type")]
	pub r#type: Vec<u8>,
	pub attributes: Vec<EventAttribute>,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub struct EventAttribute {
	pub key: Vec<u8>,
	pub value: Vec<u8>,
}

pub type CosmosEvents = Vec<CosmosEvent>;

#[derive(Clone, Debug, Default)]
pub struct EventManager {
	events: CosmosEvents,
}

impl traits::EventManager for EventManager {
	fn new() -> Self {
		Self::default()
	}

	fn events(&self) -> CosmosEvents {
		self.events.clone()
	}

	fn emit_event(&mut self, event: CosmosEvent) {
		self.events.push(event);
	}

	fn emit_events(&mut self, events: CosmosEvents) {
		self.events.extend(events);
	}
}
