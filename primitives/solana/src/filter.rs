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

use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RpcFilterType {
	DataSize(u64),
	Memcmp(Memcmp),
	TokenAccountState,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Memcmp {
	/// Data offset to begin match
	offset: usize,
	/// Bytes, encoded with specified encoding
	#[serde(flatten)]
	bytes: MemcmpEncodedBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase", tag = "encoding", content = "bytes")]
pub enum MemcmpEncodedBytes {
	Base58(String),
	Base64(String),
	Bytes(Vec<u8>),
}

impl<'de> Deserialize<'de> for MemcmpEncodedBytes {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(untagged)]
		enum DataType {
			Encoded(String),
			Raw(Vec<u8>),
		}

		#[derive(Deserialize)]
		#[serde(rename_all = "camelCase")]
		enum RpcMemcmpEncoding {
			Base58,
			Base64,
			Bytes,
		}

		#[derive(Deserialize)]
		struct RpcMemcmpInner {
			bytes: DataType,
			encoding: Option<RpcMemcmpEncoding>,
		}

		let data = RpcMemcmpInner::deserialize(deserializer)?;

		let memcmp_encoded_bytes = match data.bytes {
			DataType::Encoded(bytes) => match data.encoding.unwrap_or(RpcMemcmpEncoding::Base58) {
				RpcMemcmpEncoding::Base58 | RpcMemcmpEncoding::Bytes =>
					MemcmpEncodedBytes::Base58(bytes),
				RpcMemcmpEncoding::Base64 => MemcmpEncodedBytes::Base64(bytes),
			},
			DataType::Raw(bytes) => MemcmpEncodedBytes::Bytes(bytes),
		};

		Ok(memcmp_encoded_bytes)
	}
}
