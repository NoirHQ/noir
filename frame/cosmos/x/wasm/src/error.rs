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
