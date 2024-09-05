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

use fp_evm::TransactionValidationError;
use fp_self_contained::SelfContainedCall;
use frame_babel::{ethereum::TransactionExt, UnifyAccount};
use np_ethereum::Address as EthereumAddress;
use sp_core::ecdsa;
use sp_runtime::{
	traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf},
	transaction_validity::{
		InvalidTransaction, TransactionValidity, TransactionValidityError, UnknownTransaction,
	},
	DispatchResultWithInfo,
};

impl SelfContainedCall for RuntimeCall {
	type SignedInfo = AccountId;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => {
				if let pallet_ethereum::Call::transact { transaction } = call {
					let check = || {
						let origin = transaction
							.recover_key()
							.map(|key| Self::SignedInfo::from(key))
							.ok_or(InvalidTransaction::Custom(
								TransactionValidationError::InvalidSignature as u8,
							))?;
						Ok(origin)
					};
					Some(check())
				} else {
					None
				}
			},
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			RuntimeCall::Ethereum(call) => {
				if let pallet_ethereum::Call::transact { transaction } = call {
					if transaction.nonce() == 0 {
						if UnifyAccount::<Runtime>::unify_ecdsa(info).is_err() {
							return Some(Err(TransactionValidityError::Unknown(
								UnknownTransaction::CannotLookup,
							)))
						}
					}
				}
				let public: ecdsa::Public = info.clone().try_into().unwrap();
				let address: EthereumAddress = public.into();
				call.validate_self_contained(&address.into(), dispatch_info, len)
			},
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => {
				if let pallet_ethereum::Call::transact { transaction } = &call {
					if transaction.nonce() == 0 {
						if UnifyAccount::<Runtime>::unify_ecdsa(info).is_err() {
							return Some(Err(TransactionValidityError::Unknown(
								UnknownTransaction::CannotLookup,
							)))
						}
					}
				}
				let public: ecdsa::Public = info.clone().try_into().unwrap();
				let address: EthereumAddress = public.into();
				call.pre_dispatch_self_contained(&address.into(), dispatch_info, len)
			},
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
				let public: ecdsa::Public = info.clone().try_into().unwrap();
				let address: EthereumAddress = public.into();
				Some(call.dispatch(RuntimeOrigin::from(
					pallet_ethereum::RawOrigin::EthereumTransaction(address.into()),
				)))
			},
			_ => None,
		}
	}
}
