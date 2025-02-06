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

use pallet_cosmos_types::errors::{CosmosError, RootError};
use pallet_cosmos_x_wasm_types::errors::WasmError;
use pallet_cosmwasm::runtimes::vm::CosmwasmVMError;

pub fn handle_vm_error<T, E>(e: CosmwasmVMError<T>, default: E) -> CosmosError
where
	T: pallet_cosmwasm::Config,
	E: Into<CosmosError>,
{
	log::debug!(target: "runtime::cosmos", "{:?}", e);

	match e {
		CosmwasmVMError::OutOfGas => WasmError::GasLimit.into(),
		CosmwasmVMError::ContractNotFound => WasmError::NoSuchContract.into(),
		CosmwasmVMError::AccountConvert => RootError::InvalidAddress.into(),
		CosmwasmVMError::NotImplemented | CosmwasmVMError::Unsupported =>
			RootError::NotSupported.into(),
		CosmwasmVMError::AssetConversion => RootError::InvalidCoins.into(),
		CosmwasmVMError::ExecuteDeserialize |
		CosmwasmVMError::ExecuteSerialize |
		CosmwasmVMError::QueryDeserialize |
		CosmwasmVMError::QuerySerialize => WasmError::Invalid.into(),
		_ => default.into(),
	}
}
