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

use crate::{events::CosmosEvent, gas::Gas};
use nostd::prelude::*;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

#[derive(Clone, Decode, Encode, Debug, TypeInfo, Serialize, Deserialize)]
pub struct GasInfo {
	pub gas_wanted: Gas,
	pub gas_used: Gas,
}

#[derive(Clone, Decode, Encode, Debug, TypeInfo, Serialize, Deserialize)]
pub struct SimulateResponse {
	pub gas_info: GasInfo,
	pub events: Vec<CosmosEvent>,
}
