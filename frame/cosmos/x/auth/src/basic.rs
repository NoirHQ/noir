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

use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx;
use frame_support::{ensure, traits::Get};
use nostd::marker::PhantomData;
use pallet_cosmos_types::{
	errors::{CosmosError, RootError},
	handler::AnteDecorator,
};
use sp_runtime::SaturatedConversion;

pub struct ValidateBasicDecorator<T>(PhantomData<T>);
impl<T> AnteDecorator for ValidateBasicDecorator<T>
where
	T: frame_system::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> Result<(), CosmosError> {
		ensure!(!tx.signatures.is_empty(), RootError::NoSignatures);
		let auth_info = tx.auth_info.as_ref().ok_or(RootError::TxDecodeError)?;
		ensure!(auth_info.signer_infos.len() == tx.signatures.len(), RootError::Unauthorized);

		Ok(())
	}
}

pub struct TxTimeoutHeightDecorator<T>(PhantomData<T>);
impl<T> AnteDecorator for TxTimeoutHeightDecorator<T>
where
	T: frame_system::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> Result<(), CosmosError> {
		let body = tx.body.as_ref().ok_or(RootError::TxDecodeError)?;

		let block_number: u64 = frame_system::Pallet::<T>::block_number().saturated_into();
		if body.timeout_height > 0 && block_number > body.timeout_height {
			return Err(RootError::TxTimeoutHeightError.into());
		}

		Ok(())
	}
}

pub struct ValidateMemoDecorator<T>(PhantomData<T>);
impl<T> AnteDecorator for ValidateMemoDecorator<T>
where
	T: pallet_cosmos::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> Result<(), CosmosError> {
		let body = tx.body.as_ref().ok_or(RootError::TxDecodeError)?;
		ensure!(body.memo.len() <= T::MaxMemoCharacters::get() as usize, RootError::MemoTooLarge);

		Ok(())
	}
}
