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

#![cfg_attr(not(feature = "std"), no_std)]

use nostd::{string::String, vec::Vec};
use pallet_cosmos_types::tx::SimulateResponse;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_api::decl_runtime_apis;
use sp_runtime::traits::Block as BlockT;

#[derive(Clone, Decode, Encode, Debug, Eq, PartialEq, TypeInfo)]
pub enum SimulateError {
	InvalidTransaction,
	InternalError(Vec<u8>),
}

pub type SimulateResult = Result<SimulateResponse, SimulateError>;

#[derive(Clone, Decode, Encode, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ChainInfo {
	pub chain_id: String,
	pub bech32_prefix: String,
	pub name: String,
	pub version: String,
}

decl_runtime_apis! {
	pub trait CosmosRuntimeApi {
		fn convert_tx(tx_bytes: Vec<u8>) -> <Block as BlockT>::Extrinsic;
		fn simulate(tx_bytes: Vec<u8>) -> SimulateResult;
		fn chain_info() -> ChainInfo;
	}
}
