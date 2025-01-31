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

use crate::gas::Gas;
use cosmos_sdk_proto::cosmos::tx::v1beta1::{Fee, Tx};
use nostd::{string::String, vec::Vec};

pub trait Msg {
	// get_signers returns the addresses of signers that must sign.
	fn get_signers(self) -> Vec<String>;
}

pub trait FeeTx {
	fn fee(&self) -> Option<Fee>;
	fn gas(&self) -> Option<Gas>;
	fn fee_payer(&self) -> Option<String>;
	fn fee_granter(&self) -> Option<String>;
}

impl FeeTx for Tx {
	fn fee(&self) -> Option<Fee> {
		self.auth_info.as_ref().and_then(|auth_info| auth_info.fee.clone())
	}

	fn gas(&self) -> Option<Gas> {
		self.fee().map(|fee| fee.gas_limit)
	}

	fn fee_payer(&self) -> Option<String> {
		self.fee().map(|fee| fee.payer.clone())
	}

	fn fee_granter(&self) -> Option<String> {
		self.fee().map(|fee| fee.granter.clone())
	}
}
