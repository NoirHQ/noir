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

pub mod traits;

pub type Gas = u64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
	GasOverflow(&'static str),
	OutOfGas(&'static str),
}

#[derive(Clone, Debug)]
pub struct BasicGasMeter {
	pub limit: Gas,
	pub consumed: Gas,
}

impl traits::GasMeter for BasicGasMeter {
	fn new(limit: Gas) -> Self {
		Self { limit, consumed: 0 }
	}

	fn consumed_gas(&self) -> Gas {
		self.consumed
	}

	fn gas_remaining(&self) -> Gas {
		self.limit.saturating_sub(self.consumed)
	}

	fn limit(&self) -> Gas {
		self.limit
	}

	fn consume_gas(&mut self, amount: Gas, descriptor: &'static str) -> Result<Gas, Error> {
		let consumed = self.consumed.checked_add(amount).ok_or(Error::GasOverflow(descriptor))?;
		if consumed > self.limit {
			return Err(Error::OutOfGas(descriptor));
		}

		self.consumed = consumed;
		Ok(self.consumed)
	}
}
