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

use crate::clock::{Epoch, Slot};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EpochInfo {
	/// The current epoch
	pub epoch: Epoch,

	/// The current slot, relative to the start of the current epoch
	pub slot_index: u64,

	/// The number of slots in this epoch
	pub slots_in_epoch: u64,

	/// The absolute current slot
	pub absolute_slot: Slot,

	/// The current block height
	pub block_height: u64,

	/// Total number of transactions processed without error since genesis
	pub transaction_count: Option<u64>,
}
