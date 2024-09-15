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
use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx;
use frame_support::{ensure, pallet_prelude::ValidTransaction, traits::Get};
use pallet_cosmos_types::handler::AnteDecorator;
use sp_runtime::{
	transaction_validity::{InvalidTransaction, TransactionValidity},
	SaturatedConversion,
};

pub struct ValidateBasicDecorator<T>(PhantomData<T>);

impl<T> AnteDecorator for ValidateBasicDecorator<T>
where
	T: frame_system::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> TransactionValidity {
		ensure!(!tx.signatures.is_empty(), InvalidTransaction::Call);
		let auth_info = tx.auth_info.as_ref().ok_or(InvalidTransaction::Call)?;
		ensure!(auth_info.signer_infos.len() == tx.signatures.len(), InvalidTransaction::Call);

		Ok(ValidTransaction::default())
	}
}

pub struct TxTimeoutHeightDecorator<T>(PhantomData<T>);

impl<T> AnteDecorator for TxTimeoutHeightDecorator<T>
where
	T: frame_system::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> TransactionValidity {
		let body = tx.body.as_ref().ok_or(InvalidTransaction::Call)?;

		let block_number: u64 = frame_system::Pallet::<T>::block_number().saturated_into();
		if body.timeout_height > 0 && block_number > body.timeout_height {
			return Err(InvalidTransaction::Stale.into());
		}

		Ok(ValidTransaction::default())
	}
}

pub struct ValidateMemoDecorator<T>(PhantomData<T>);

impl<T> AnteDecorator for ValidateMemoDecorator<T>
where
	T: pallet_cosmos::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> TransactionValidity {
		let body = tx.body.as_ref().ok_or(InvalidTransaction::Call)?;
		ensure!(body.memo.len() <= T::MaxMemoCharacters::get() as usize, InvalidTransaction::Call);

		Ok(ValidTransaction::default())
	}
}
