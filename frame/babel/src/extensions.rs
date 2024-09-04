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

use core::marker::PhantomData;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{DispatchInfoOf, One, SignedExtension, Zero},
	transaction_validity::{TransactionValidityError, ValidTransaction},
};

/// Integrates the accounts associated with the same public key.
///
/// WARN: This extension should be placed after the `CheckNonce` extension.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnifyAccount<T>(PhantomData<fn() -> T>);

impl<T> UnifyAccount<T> {
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

impl<T: Config> UnifyAccount<T> {
	pub fn unify_ecdsa(who: &T::AccountId) -> Result<(), ()> {
		if let Ok(public) = who.clone().try_into() {
			#[cfg(feature = "ethereum")]
			{
				let address = Address::ethereum(public);
				T::AddressMap::try_insert(who.clone(), address).map_err(|_| ())?;
			}
		}
		Ok(())
	}
}

impl<T> Clone for UnifyAccount<T> {
	fn clone(&self) -> Self {
		Self(Default::default())
	}
}

impl<T> PartialEq for UnifyAccount<T> {
	fn eq(&self, _: &Self) -> bool {
		true
	}
}

impl<T> Eq for UnifyAccount<T> {}

impl<T> core::fmt::Debug for UnifyAccount<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "UnifyAccount")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut core::fmt::Formatter) -> core::fmt::Result {
		Ok(())
	}
}

impl<T: Config> SignedExtension for UnifyAccount<T> {
	type AccountId = T::AccountId;
	type Call = T::RuntimeCall;
	type AdditionalSigned = ();
	type Pre = ();
	const IDENTIFIER: &'static str = "UnifyAccount";

	fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
		Ok(())
	}

	fn validate(
		&self,
		who: &Self::AccountId,
		_: &Self::Call,
		_: &DispatchInfoOf<Self::Call>,
		_: usize,
	) -> Result<ValidTransaction, TransactionValidityError> {
		let account = frame_system::Account::<T>::get(who);
		if account.nonce.is_zero() {
			let _ = Self::unify_ecdsa(who);
		}
		Ok(ValidTransaction::default())
	}

	fn pre_dispatch(
		self,
		who: &Self::AccountId,
		_: &Self::Call,
		_: &DispatchInfoOf<Self::Call>,
		_: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		let account = frame_system::Account::<T>::get(who);
		if account.nonce.is_one() {
			let _ = Self::unify_ecdsa(who);
		}
		Ok(())
	}
}
