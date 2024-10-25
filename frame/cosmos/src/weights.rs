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

use core::marker::PhantomData;
use frame_support::{dispatch::DispatchClass, weights::Weight};
use frame_system::limits::BlockWeights;
use sp_core::Get;

pub trait WeightInfo {
	fn base_weight() -> Weight;
}

impl WeightInfo for () {
	fn base_weight() -> Weight {
		BlockWeights::default().per_class.get(DispatchClass::Normal).base_extrinsic
	}
}

pub struct CosmosWeight<T>(PhantomData<T>);
impl<T> WeightInfo for CosmosWeight<T>
where
	T: frame_system::Config,
{
	fn base_weight() -> Weight {
		T::BlockWeights::get().get(DispatchClass::Normal).base_extrinsic
	}
}
