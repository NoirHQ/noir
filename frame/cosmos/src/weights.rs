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

use frame_support::{dispatch::DispatchClass, weights::Weight};
use frame_system::limits::BlockWeights;
use nostd::marker::PhantomData;
use sp_core::Get;

pub trait WeightInfo {
	fn base_weight() -> Weight;
}

impl WeightInfo for () {
	fn base_weight() -> Weight {
		BlockWeights::default().per_class.get(DispatchClass::Normal).base_extrinsic
	}
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T> WeightInfo for SubstrateWeight<T>
where
	T: frame_system::Config,
{
	fn base_weight() -> Weight {
		T::BlockWeights::get().get(DispatchClass::Normal).base_extrinsic
	}
}
