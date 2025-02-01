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

use sp_blockchain::Result;
use sp_runtime::DigestItem;
use std::sync::Arc;

/// A trait that provides multiple pre-runtime digests for different consensus engines.
#[async_trait::async_trait]
pub trait PreDigestProvider {
	/// Returns a set of pre-runtime digests.
	async fn pre_digest(&self, best_hash: &[u8]) -> Result<Vec<DigestItem>>;
}

#[async_trait::async_trait]
impl<T> PreDigestProvider for Arc<T>
where
	T: PreDigestProvider + Send + Sync,
{
	async fn pre_digest(&self, best_hash: &[u8]) -> Result<Vec<DigestItem>> {
		self.as_ref().pre_digest(best_hash).await
	}
}

#[async_trait::async_trait]
impl PreDigestProvider for () {
	async fn pre_digest(&self, _best_hash: &[u8]) -> Result<Vec<DigestItem>> {
		Ok(vec![])
	}
}
