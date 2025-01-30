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
