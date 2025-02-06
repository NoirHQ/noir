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

use crate::{context::traits::Context, errors::CosmosError};
use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx;

pub trait AnteDecorator {
	fn ante_handle(tx: &Tx, simulate: bool) -> Result<(), CosmosError>;
}

impl AnteDecorator for () {
	fn ante_handle(_tx: &Tx, _simulate: bool) -> Result<(), CosmosError> {
		Ok(())
	}
}

#[impl_trait_for_tuples::impl_for_tuples(1, 12)]
impl AnteDecorator for Tuple {
	fn ante_handle(tx: &Tx, simulate: bool) -> Result<(), CosmosError> {
		for_tuples!( #( Tuple::ante_handle(tx, simulate)?; )* );
		Ok(())
	}
}

pub trait PostDecorator<C: Context> {
	fn post_handle(ctx: &mut C, tx: &Tx, simulate: bool) -> Result<(), CosmosError>;
}

impl<C: Context> PostDecorator<C> for () {
	fn post_handle(_ctx: &mut C, _tx: &Tx, _simulate: bool) -> Result<(), CosmosError> {
		Ok(())
	}
}

#[impl_trait_for_tuples::impl_for_tuples(1, 12)]
impl<C: Context> PostDecorator<C> for Tuple {
	fn post_handle(ctx: &mut C, tx: &Tx, simulate: bool) -> Result<(), CosmosError> {
		for_tuples!( #( Tuple::post_handle(ctx, tx, simulate)?; )* );
		Ok(())
	}
}
