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
