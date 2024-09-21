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

use crate::gas::Gas;
use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx;

pub fn get_gas_limit(tx: &Tx) -> Option<Gas> {
	tx.auth_info
		.as_ref()
		.and_then(|auth_info| auth_info.fee.as_ref())
		.map(|fee| fee.gas_limit)
}
