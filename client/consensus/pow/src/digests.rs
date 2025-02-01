// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use nc_consensus::PreDigestProvider;
use np_consensus_pow::POW_ENGINE_ID;
use parity_scale_codec::{Decode, Encode};
use sp_blockchain::Result;
use sp_runtime::DigestItem;
use std::ops::Deref;

/// Generic pre-digest for PoW consensus engine.
#[derive(Clone, Debug, Decode, Encode)]
pub struct PreDigest<AccountId, Inner = ()> {
	author: AccountId,
	inner: Inner,
}

impl<AccountId, Inner> PreDigest<AccountId, Inner> {
	pub fn new(author: AccountId, inner: Inner) -> Self {
		Self { author, inner }
	}

	pub fn author(&self) -> &AccountId {
		&self.author
	}

	pub fn into_inner(self) -> Inner {
		self.inner
	}
}

impl<AccountId, Inner> Deref for PreDigest<AccountId, Inner> {
	type Target = Inner;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

#[async_trait::async_trait]
impl<AccountId> PreDigestProvider for PreDigest<AccountId, ()>
where
	AccountId: Encode + Send + Sync,
{
	async fn pre_digest(&self, _best_hash: &[u8]) -> Result<Vec<DigestItem>> {
		Ok(vec![DigestItem::PreRuntime(POW_ENGINE_ID, self.author.encode())])
	}
}
