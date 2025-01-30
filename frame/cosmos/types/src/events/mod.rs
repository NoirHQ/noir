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
