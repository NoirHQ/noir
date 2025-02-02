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

use alloc::collections::BTreeMap;
use parity_scale_codec::Encode;
use sp_inherents::{self, InherentData, InherentIdentifier, IsFatalError};
use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use {core::marker::PhantomData, parity_scale_codec::Decode, sp_runtime::traits::One};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"rewards_";

#[derive(Encode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	/// Invalid reward amount.
	#[cfg_attr(feature = "std", error("Invalid reward amount"))]
	InvalidReward,
	/// Duplicate reward beneficiary.
	#[cfg_attr(feature = "std", error("Duplicate reward beneficiary"))]
	DuplicateBeneficiary,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

pub type InherentType<AccountId, Share> = BTreeMap<AccountId, Share>;

#[cfg(feature = "std")]
pub struct InherentDataProvider<AccountId, Share> {
	pub author: AccountId,
	_marker: PhantomData<Share>,
}

#[cfg(feature = "std")]
impl<AccountId, Share> InherentDataProvider<AccountId, Share> {
	pub fn new(author: AccountId) -> Self {
		Self { author, _marker: PhantomData }
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl<AccountId, Share> sp_inherents::InherentDataProvider for InherentDataProvider<AccountId, Share>
where
	AccountId: Clone + Ord + Encode + Send + Sync,
	Share: One + Encode + Send + Sync,
{
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(
			INHERENT_IDENTIFIER,
			&InherentType::from([(self.author.clone(), Share::one())]),
		)
	}

	async fn try_handle_error(
		&self,
		_identifier: &InherentIdentifier,
		_error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		None
	}
}
