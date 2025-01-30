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
use frame_support::{ensure, traits::Contains};
use nostd::marker::PhantomData;
use pallet_cosmos_types::{
	errors::{CosmosError, RootError},
	handler::AnteDecorator,
};

pub struct KnownMsgDecorator<T>(PhantomData<T>);
impl<T> AnteDecorator for KnownMsgDecorator<T>
where
	T: pallet_cosmos::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> Result<(), CosmosError> {
		let body = tx.body.as_ref().ok_or(RootError::TxDecodeError)?;

		ensure!(body.messages.iter().all(T::MsgFilter::contains), RootError::TxDecodeError);

		Ok(())
	}
}
