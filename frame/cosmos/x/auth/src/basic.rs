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
