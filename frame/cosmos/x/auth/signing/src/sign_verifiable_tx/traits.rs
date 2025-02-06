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

use super::SigVerifiableTxError;
use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx;
use nostd::{string::String, vec::Vec};

pub trait SigVerifiableTx {
	fn get_signers(tx: &Tx) -> Result<Vec<String>, SigVerifiableTxError>;
	fn fee_payer(tx: &Tx) -> Result<String, SigVerifiableTxError>;
	fn sequence(tx: &Tx) -> Result<u64, SigVerifiableTxError>;
}
