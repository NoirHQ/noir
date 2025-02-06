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

use frame_support::{pallet_prelude::TransactionValidityError, PalletError};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::transaction_validity::InvalidTransaction;

#[derive(Clone, Debug, PartialEq, Eq, Decode, Encode, TypeInfo, PalletError)]
pub struct CosmosError {
	pub codespace: u8,
	pub code: u8,
}

impl From<CosmosError> for TransactionValidityError {
	fn from(e: CosmosError) -> Self {
		TransactionValidityError::Invalid(InvalidTransaction::Custom(e.code))
	}
}

pub const ROOT_CODESPACE: u8 = 0;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RootError {
	TxDecodeError = 2,
	Unauthorized = 4,
	InsufficientFunds = 5,
	UnknownRequest = 6,
	InvalidAddress = 7,
	InvalidPubKey = 8,
	UnknownAddress = 9,
	InvalidCoins = 10,
	OutOfGas = 11,
	MemoTooLarge = 12,
	InsufficientFee = 13,
	TooManySignatures = 14,
	NoSignatures = 15,
	InvalidRequest = 18,
	InvalidSigner = 24,
	TxTimeoutHeightError = 30,
	WrongSequence = 32,
	UnpackAnyError = 34,
	NotSupported = 37,
	InvalidGasLimit = 41,
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
