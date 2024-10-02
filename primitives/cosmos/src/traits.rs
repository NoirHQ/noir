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

/// Chain information trait.
pub trait ChainInfo {
	fn bech32_prefix() -> &'static str;
	fn chain_id() -> &'static str;
}

/// Cosmos Hub chain information.
pub struct CosmosHub;

impl ChainInfo for CosmosHub {
	fn bech32_prefix() -> &'static str {
		"cosmos"
	}
	fn chain_id() -> &'static str {
		"ziggurat"
	}
}
