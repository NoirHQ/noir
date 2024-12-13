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

#[cfg(feature = "scale")]
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "scale")]
use scale_info::TypeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
pub struct CommitmentConfig {
	pub commitment: CommitmentLevel,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "scale", derive(Encode, Decode, TypeInfo))]
#[derive(Debug, Clone)]
/// An attribute of a slot. It describes how finalized a block is at some point in time. For
/// example, a slot is said to be at the max level immediately after the cluster recognizes the
/// block at that slot as finalized. When querying the ledger state, use lower levels of commitment
/// to report progress and higher levels to ensure state changes will not be rolled back.
pub enum CommitmentLevel {
	/// The highest slot of the heaviest fork processed by the node. Ledger state at this slot is
	/// not derived from a confirmed or finalized block, but if multiple forks are present, is from
	/// the fork the validator believes is most likely to finalize.
	Processed,

	/// The highest slot that has been voted on by supermajority of the cluster, ie. is confirmed.
	/// Confirmation incorporates votes from gossip and replay. It does not count votes on
	/// descendants of a block, only direct votes on that block, and upholds "optimistic
	/// confirmation" guarantees in release 1.3 and onwards.
	Confirmed,

	/// The highest slot having reached max vote lockout, as recognized by a supermajority of the
	/// cluster.
	Finalized,
}
