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

use core::marker::PhantomData;
use cosmos_sdk_proto::cosmos::tx::v1beta1::{Fee, Tx};
use frame_support::{
	ensure,
	traits::{
		fungibles::Balanced,
		tokens::{Fortitude, Precision, Preservation},
		Currency, ExistenceRequirement, WithdrawReasons,
	},
};
use nostd::vec;
use pallet_cosmos::AddressMapping;
use pallet_cosmos_types::{
	address::{acc_address_from_bech32, AUTH_ADDRESS_LEN},
	coin::traits::Coins,
	context::traits::MinGasPrices,
	errors::{CosmosError, RootError},
	events::{
		CosmosEvent, EventAttribute, ATTRIBUTE_KEY_FEE, ATTRIBUTE_KEY_FEE_PAYER, EVENT_TYPE_TX,
	},
	handler::AnteDecorator,
	tx_msgs::FeeTx,
};
use pallet_cosmos_x_auth_signing::sign_verifiable_tx::traits::SigVerifiableTx;
use sp_core::{Get, H160};
use sp_runtime::{
	traits::{TryConvertBack, Zero},
	SaturatedConversion,
};

pub struct DeductFeeDecorator<T>(PhantomData<T>);
impl<T> AnteDecorator for DeductFeeDecorator<T>
where
	T: frame_system::Config + pallet_cosmos::Config,
{
	fn ante_handle(tx: &Tx, simulate: bool) -> Result<(), CosmosError> {
		let gas_limit = tx.gas().ok_or(RootError::TxDecodeError)?;
		if !simulate && !frame_system::Pallet::<T>::block_number().is_zero() && gas_limit == 0 {
			return Err(RootError::InvalidGasLimit.into());
		}

		if !simulate {
			let fee_coins = tx.fee().ok_or(RootError::TxDecodeError)?.amount;
			let min_prices = T::MinGasPrices::min_prices();
			for gp in min_prices.iter() {
				let fee =
					gp.amount.checked_mul(gas_limit.into()).ok_or(RootError::InsufficientFee)?;
				let amount =
					fee_coins.amount_of(&gp.denom).map_err(|_| RootError::InsufficientFee)?;

				ensure!(amount >= fee && fee != 0, RootError::InsufficientFee);
			}
		}

		Self::check_deduct_fee(tx)?;

		Ok(())
	}
}

impl<T> DeductFeeDecorator<T>
where
	T: pallet_cosmos::Config,
{
	fn check_deduct_fee(tx: &Tx) -> Result<(), CosmosError> {
		let fee_payer = T::SigVerifiableTx::fee_payer(tx).map_err(|_| RootError::TxDecodeError)?;
		let fee = tx.fee().ok_or(RootError::TxDecodeError)?;

		// Fee granter not supported
		ensure!(fee.granter.is_empty(), RootError::InvalidRequest);

		let (_hrp, address_raw) =
			acc_address_from_bech32(&fee_payer).map_err(|_| RootError::InvalidAddress)?;
		ensure!(address_raw.len() == AUTH_ADDRESS_LEN, RootError::InvalidAddress);
		let deduct_fees_from = T::AddressMapping::into_account_id(H160::from_slice(&address_raw));

		if !fee.amount.is_empty() {
			Self::deduct_fees(&deduct_fees_from, &fee)?;
		}

		pallet_cosmos::Pallet::<T>::deposit_event(pallet_cosmos::Event::AnteHandled(vec![
			CosmosEvent {
				r#type: EVENT_TYPE_TX.into(),
				attributes: vec![
					EventAttribute {
						key: ATTRIBUTE_KEY_FEE.into(),
						value: fee.amount.to_string().into(),
					},
					EventAttribute { key: ATTRIBUTE_KEY_FEE_PAYER.into(), value: fee_payer.into() },
				],
			},
		]));

		Ok(())
	}

	fn deduct_fees(acc: &T::AccountId, fee: &Fee) -> Result<(), CosmosError> {
		for amt in fee.amount.iter() {
			let amount = amt.amount.parse::<u128>().map_err(|_| RootError::InsufficientFee)?;

			if amt.denom == T::NativeDenom::get() {
				let _imbalance = T::NativeAsset::withdraw(
					acc,
					amount.saturated_into(),
					WithdrawReasons::TRANSACTION_PAYMENT,
					ExistenceRequirement::KeepAlive,
				)
				.map_err(|_| RootError::InsufficientFunds)?;

				// TODO: Resolve imbalance
			} else {
				let asset_id = T::AssetToDenom::try_convert_back(amt.denom.clone())
					.map_err(|_| RootError::InsufficientFunds)?;
				let _imbalance = T::Assets::withdraw(
					asset_id,
					acc,
					amount.saturated_into(),
					Precision::Exact,
					Preservation::Preserve,
					Fortitude::Polite,
				)
				.map_err(|_| RootError::InsufficientFunds)?;

				// TODO: Resolve imbalance
			}
		}

		Ok(())
	}
}
