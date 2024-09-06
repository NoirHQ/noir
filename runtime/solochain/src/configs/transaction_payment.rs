// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
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

use crate::*;

use common::{units::CENTS, SlowAdjustingFeeUpdate};
use frame_support::weights::{
	constants::ExtrinsicBaseWeight, ConstantMultiplier, WeightToFeeCoefficient,
	WeightToFeeCoefficients, WeightToFeePolynomial,
};
use pallet_transaction_payment::FungibleAdapter;
use smallvec::smallvec;
use sp_runtime::Perbill;

parameter_types! {
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const TransactionByteFee: Balance = (1 * CENTS) / 100;
}

pub struct WeightToFee;

impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Balance> {
		let p = 1 * CENTS;
		let q = 10 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![WeightToFeeCoefficient {
			coeff_integer: p / q,
			coeff_frac: Perbill::from_rational(p % q, q),
			negative: false,
			degree: 1,
		}]
	}
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, () /* TODO: DealWithFees */>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
}
