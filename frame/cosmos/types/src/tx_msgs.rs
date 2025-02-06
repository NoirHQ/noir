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
