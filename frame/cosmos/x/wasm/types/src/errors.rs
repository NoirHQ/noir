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

use pallet_cosmos_types::errors::CosmosError;

pub const WASM_CODESPACE: u8 = 1;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WasmError {
	CreateFailed = 2,
	InstantiateFailed = 4,
	ExecuteFailed = 5,
	GasLimit = 6,
	MigrationFailed = 11,
	Empty = 12,
	Invalid = 14,
	NoSuchContract = 22,
}

impl From<WasmError> for CosmosError {
	fn from(error: WasmError) -> Self {
		CosmosError { codespace: WASM_CODESPACE, code: error as u8 }
	}
}

#[cfg(test)]
mod tests {
	use super::{CosmosError, WasmError};
	use crate::errors::WASM_CODESPACE;

	#[test]
	fn wasm_error_test() {
		let error: CosmosError = WasmError::CreateFailed.into();
		assert_eq!(
			error,
			CosmosError { codespace: WASM_CODESPACE, code: WasmError::CreateFailed as u8 }
		);
	}
}
