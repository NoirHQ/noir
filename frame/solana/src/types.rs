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

use crate::{BalanceOf, Config};
use frame_support::sp_runtime::traits::{CheckedAdd, CheckedMul, CheckedSub, Get, Saturating};
use frame_system::pallet_prelude::BlockNumberFor;
use nostd::cmp::Ordering;
use np_runtime::traits::LossyInto;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use solana_sdk::{
	clock::Epoch, fee_calculator::FeeCalculator, instruction::InstructionError, pubkey::Pubkey,
};

#[derive(Decode, Encode, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct HashInfo<T: Config> {
	pub fee_calculator: FeeCalculator,
	pub hash_index: BlockNumberFor<T>,
	pub timestamp: T::Moment,
}

impl<T: Config> HashInfo<T> {
	pub fn lamports_per_signature(&self) -> u64 {
		self.fee_calculator.lamports_per_signature
	}
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Decode, Encode, MaxEncodedLen, TypeInfo)]
pub struct AccountMetadata {
	/// the epoch at which this account will next owe rent
	pub rent_epoch: Epoch,
	/// the program that owns this account. If executable, the program that loads this account.
	pub owner: Pubkey,
	/// this account's data contains a loaded program (and is now read-only)
	pub executable: bool,
}

#[derive(Clone, PartialEq, Eq, Decode, Encode, MaxEncodedLen, TypeInfo)]
#[derive_where(Copy, Debug)]
pub struct Lamports<T: Config>(BalanceOf<T>);

impl<T: Config> core::fmt::Display for Lamports<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "{}.{}", self.get(), (self.0 % T::DecimalMultiplier::get()).lossy_into())
	}
}

impl<T: Config> Default for Lamports<T> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<T: Config> PartialOrd for Lamports<T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl<T: Config> Ord for Lamports<T> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.0.cmp(&other.0)
	}
}

impl<T: Config> core::ops::Add for Lamports<T> {
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		Self(self.0 + rhs.0)
	}
}

impl<T: Config> core::ops::Sub for Lamports<T> {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self::Output {
		Self(self.0 - rhs.0)
	}
}

impl<T: Config> core::ops::Add<u64> for Lamports<T> {
	type Output = Self;

	fn add(self, rhs: u64) -> Self::Output {
		self + Self::from(rhs)
	}
}

impl<T: Config> core::ops::Sub<u64> for Lamports<T> {
	type Output = Self;

	fn sub(self, rhs: u64) -> Self::Output {
		self - Self::from(rhs)
	}
}

impl<T: Config> core::ops::Mul<u64> for Lamports<T> {
	type Output = Self;

	fn mul(self, rhs: u64) -> Self::Output {
		Self(self.0 * rhs.into())
	}
}

impl<T: Config> core::ops::Div<u64> for Lamports<T> {
	type Output = Self;

	fn div(self, rhs: u64) -> Self::Output {
		Self(self.0 / rhs.into())
	}
}

impl<T: Config> From<u64> for Lamports<T> {
	fn from(lamports: u64) -> Self {
		Self::checked_new(<BalanceOf<T>>::from(lamports) * T::DecimalMultiplier::get()).unwrap()
	}
}

impl<T: Config> Lamports<T> {
	pub const fn new(value: BalanceOf<T>) -> Self {
		Self(value)
	}

	pub fn checked_new(value: BalanceOf<T>) -> Option<Self> {
		let max = <BalanceOf<T>>::from(u64::MAX).checked_mul(&T::DecimalMultiplier::get())?;
		(value <= max).then_some(Self(value))
	}

	pub fn get(&self) -> u64 {
		(self.0 / T::DecimalMultiplier::get()).lossy_into()
	}

	pub fn inner(&self) -> BalanceOf<T> {
		self.0
	}

	pub fn into_inner(self) -> BalanceOf<T> {
		self.0
	}

	pub fn checked_add(&self, rhs: u64) -> Option<Self> {
		let rhs = <BalanceOf<T>>::from(rhs);
		let rhs = rhs.checked_mul(&T::DecimalMultiplier::get())?;
		self.0.checked_add(&rhs).map(Self::checked_new)?
	}

	pub fn checked_sub(&self, rhs: u64) -> Option<Self> {
		let rhs = <BalanceOf<T>>::from(rhs);
		let rhs = rhs.checked_mul(&T::DecimalMultiplier::get())?;
		self.0.checked_sub(&rhs).map(Self::checked_new)?
	}

	pub fn saturating_add(&self, rhs: u64) -> Self {
		let rhs = <BalanceOf<T>>::from(rhs);
		let rhs = rhs.saturating_mul(T::DecimalMultiplier::get());
		Self(self.0.saturating_add(rhs))
	}

	pub fn saturating_sub(&self, rhs: u64) -> Self {
		let rhs = <BalanceOf<T>>::from(rhs);
		let rhs = rhs.saturating_mul(T::DecimalMultiplier::get());
		Self(self.0.saturating_sub(rhs))
	}

	pub fn checked_add_lamports(&self, rhs: Self) -> Option<Self> {
		self.0.checked_add(&rhs.0).map(Self::checked_new)?
	}

	pub fn checked_sub_lamports(&self, rhs: Self) -> Option<Self> {
		self.0.checked_sub(&rhs.0).map(Self::checked_new)?
	}
}

pub fn checked_add<T: Config>(
	a: Lamports<T>,
	b: Lamports<T>,
) -> Result<Lamports<T>, InstructionError> {
	a.checked_add_lamports(b).ok_or(InstructionError::InsufficientFunds)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::Test;

	#[test]
	fn lamports_works() {
		let lamports = <Lamports<Test>>::checked_new((u64::MAX as u128) * 10u128.pow(10));
		assert!(lamports.is_none());

		let lamports = <Lamports<Test>>::new(1_000_001_234);
		assert_eq!(lamports.get(), 1);

		let lamports = lamports.checked_add(1).unwrap();
		assert_eq!(lamports.get(), 2);
		assert_eq!(lamports.into_inner(), 2_000_001_234);
	}
}
