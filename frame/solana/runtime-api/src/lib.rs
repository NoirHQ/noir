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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod account;
pub mod balance;
pub mod error;
pub mod fee;
pub mod transaction;

use error::Error;
use nostd::prelude::*;
use sp_api::decl_runtime_apis;

decl_runtime_apis! {
	pub trait SolanaRuntimeApi {
		fn call(method: String, params: Vec<u8>) -> Result<Vec<u8>, Error>;
	}
}

pub trait SolanaRuntimeCall<I = (), O = ()>
where
	I: for<'de> serde::Deserialize<'de> + Send + Sync,
	O: serde::Serialize + Send + Sync,
{
	fn call_raw(params: Vec<u8>) -> Result<Vec<u8>, Error> {
		let input: I = bincode::deserialize(&params).map_err(|_| Error::ParseError)?;
		let output = Self::call(input)?;
		bincode::serialize(&output).map_err(|_| Error::ParseError)
	}

	fn call(input: I) -> Result<O, Error>;
}
