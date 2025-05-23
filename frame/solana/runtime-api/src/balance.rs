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

use crate::{error::Error, SolanaRuntimeCall};
use nostd::marker::PhantomData;
use pallet_solana::Pubkey;

pub struct Balance<T>(PhantomData<T>);
impl<T> SolanaRuntimeCall<Pubkey, u64> for Balance<T>
where
	T: pallet_solana::Config,
{
	fn call(pubkey: Pubkey) -> Result<u64, Error> {
		Ok(pallet_solana::Pallet::<T>::get_balance(pubkey))
	}
}
