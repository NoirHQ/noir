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
