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

use frame_support::PalletError;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Debug, PartialEq, Eq, Decode, Encode, TypeInfo, PalletError)]
pub struct CosmosError {
	pub codespace: u8,
	pub code: u8,
}

pub const ROOT_CODESPACE: u8 = 0;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RootError {
	TxDecodeError = 2,
	InsufficientFunds = 5,
	UnknownRequest = 6,
	InvalidAddress = 7,
	InvalidCoins = 10,
	OutOfGas = 11,
	UnpackAnyError = 34,
}

impl From<RootError> for CosmosError {
	fn from(error: RootError) -> Self {
		CosmosError { codespace: ROOT_CODESPACE, code: error as u8 }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::errors::ROOT_CODESPACE;

	#[test]
	fn cosmos_error_test() {
		let error: CosmosError = RootError::InvalidAddress.into();
		assert_eq!(
			error,
			CosmosError { codespace: ROOT_CODESPACE, code: RootError::InvalidAddress as u8 }
		);
	}
}
