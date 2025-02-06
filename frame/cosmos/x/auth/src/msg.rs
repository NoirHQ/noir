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
