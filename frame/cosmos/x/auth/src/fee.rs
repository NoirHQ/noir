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

use core::marker::PhantomData;
use cosmos_sdk_proto::cosmos::tx::v1beta1::{Fee, Tx};
use frame_support::{
	ensure,
	pallet_prelude::{InvalidTransaction, TransactionValidity, ValidTransaction},
	traits::{
		fungibles::Balanced,
		tokens::{Fortitude, Precision, Preservation},
		Currency, ExistenceRequirement, WithdrawReasons,
	},
};
use pallet_cosmos::AddressMapping;
use pallet_cosmos_types::{
	address::acc_address_from_bech32,
	coin::amount_to_string,
	events::{
		CosmosEvent, EventAttribute, ATTRIBUTE_KEY_FEE, ATTRIBUTE_KEY_FEE_PAYER, EVENT_TYPE_TX,
	},
	handler::AnteDecorator,
};
use pallet_cosmos_x_auth_signing::sign_verifiable_tx::traits::SigVerifiableTx;
use sp_core::{Get, H160};
use sp_runtime::{
	traits::{Convert, Zero},
	SaturatedConversion,
};

pub struct DeductFeeDecorator<T>(PhantomData<T>);

impl<T> AnteDecorator for DeductFeeDecorator<T>
where
	T: frame_system::Config + pallet_cosmos::Config,
{
	fn ante_handle(tx: &Tx, simulate: bool) -> TransactionValidity {
		let fee = tx
			.auth_info
			.as_ref()
			.and_then(|auth_info| auth_info.fee.as_ref())
			.ok_or(InvalidTransaction::Call)?;

		if !simulate && !frame_system::Pallet::<T>::block_number().is_zero() && fee.gas_limit == 0 {
			return Err(InvalidTransaction::Call.into());
		}

		// TODO: Implements txFeeChecker

		Self::check_deduct_fee(tx)?;

		Ok(ValidTransaction::default())
	}
}

impl<T> DeductFeeDecorator<T>
where
	T: pallet_cosmos::Config,
{
	fn check_deduct_fee(tx: &Tx) -> TransactionValidity {
		let fee_payer = T::SigVerifiableTx::fee_payer(tx).map_err(|_| InvalidTransaction::Call)?;

		let fee = tx
			.auth_info
			.as_ref()
			.and_then(|auth_info| auth_info.fee.as_ref())
			.ok_or(InvalidTransaction::Call)?;

		// TODO: Fee granter support
		ensure!(fee.granter.is_empty(), InvalidTransaction::Call);

		let (_hrp, address_raw) =
			acc_address_from_bech32(&fee_payer).map_err(|_| InvalidTransaction::BadSigner)?;
		ensure!(address_raw.len() == 20, InvalidTransaction::BadSigner);
		let deduct_fees_from = T::AddressMapping::into_account_id(H160::from_slice(&address_raw));

		// TODO: Check fee is zero
		if !fee.amount.is_empty() {
			Self::deduct_fees(&deduct_fees_from, fee)?;
		}

		pallet_cosmos::Pallet::<T>::deposit_event(pallet_cosmos::Event::AnteHandled(vec![
			CosmosEvent {
				r#type: EVENT_TYPE_TX.into(),
				attributes: vec![
					EventAttribute {
						key: ATTRIBUTE_KEY_FEE.into(),
						value: amount_to_string(&fee.amount).into(),
					},
					EventAttribute { key: ATTRIBUTE_KEY_FEE_PAYER.into(), value: fee_payer.into() },
				],
			},
		]));

		Ok(ValidTransaction::default())
	}

	fn deduct_fees(acc: &T::AccountId, fee: &Fee) -> TransactionValidity {
		for amt in fee.amount.iter() {
			let amount = amt.amount.parse::<u128>().map_err(|_| InvalidTransaction::Call)?;

			if amt.denom == T::NativeDenom::get() {
				let _imbalance = T::NativeAsset::withdraw(
					acc,
					amount.saturated_into(),
					WithdrawReasons::TRANSACTION_PAYMENT,
					ExistenceRequirement::KeepAlive,
				)
				.map_err(|_| InvalidTransaction::Payment)?;

				// TODO: Resolve imbalance
			} else {
				let asset_id = T::AssetToDenom::convert(amt.denom.clone())
					.map_err(|_| InvalidTransaction::Call)?;
				let _imbalance = T::Assets::withdraw(
					asset_id,
					acc,
					amount.saturated_into(),
					Precision::Exact,
					Preservation::Preserve,
					Fortitude::Polite,
				)
				.map_err(|_| InvalidTransaction::Payment)?;

				// TODO: Resolve imbalance
			}
		}

		Ok(ValidTransaction::default())
	}
}
