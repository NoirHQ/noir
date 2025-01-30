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

//! The Multimap Pallet
//!
//! The Multimap pallet provides a map container that supports multiple
//! values with the same key.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unreachable_patterns)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod traits;

use alloc::vec::Vec;
use core::{fmt::Debug, marker::PhantomData};
use frame_support::{
	sp_runtime::traits::MaybeSerializeDeserialize, BoundedBTreeSet, StorageHasher,
};
use parity_scale_codec::{FullCodec, MaxEncodedLen};
use scale_info::TypeInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[pallet::config(with_default)]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// Type of the keys.
		#[pallet::no_default]
		type Key: Clone
			+ Debug
			+ PartialEq
			+ FullCodec
			+ MaxEncodedLen
			+ TypeInfo
			+ MaybeSerializeDeserialize;

		/// Storage hasher for the keys.
		type KeyHasher: StorageHasher;

		/// Type of the values.
		#[pallet::no_default]
		type Value: Clone + Ord + FullCodec + MaxEncodedLen + TypeInfo + MaybeSerializeDeserialize;

		/// Storage hasher for the values.
		type ValueHasher: StorageHasher;

		/// Maximum number of items that can be stored per key.
		#[pallet::constant]
		type CapacityPerKey: Get<u32>;
	}

	pub mod config_preludes {
		use super::*;
		use frame_support::derive_impl;

		/// A configuration for testing.
		pub struct TestDefaultConfig;

		#[derive_impl(frame_system::config_preludes::TestDefaultConfig, no_aggregated_types)]
		impl frame_system::DefaultConfig for TestDefaultConfig {}

		#[frame_support::register_default_impl(TestDefaultConfig)]
		impl DefaultConfig for TestDefaultConfig {
			type KeyHasher = Twox64Concat;
			type ValueHasher = Twox64Concat;
			type CapacityPerKey = ConstU32<1>;
		}
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		CapacityOverflow,
		DuplicateValue,
	}

	#[pallet::storage]
	#[pallet::getter(fn get)]
	pub type Map<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		T::KeyHasher,
		T::Key,
		BoundedBTreeSet<T::Value, T::CapacityPerKey>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn find_key)]
	pub type Index<T: Config<I>, I: 'static = ()> = StorageMap<_, T::ValueHasher, T::Value, T::Key>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		multimap: Vec<(T::Key, Vec<T::Value>)>,
	}

	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			GenesisConfig { multimap: Vec::new() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> BuildGenesisConfig for GenesisConfig<T, I> {
		fn build(&self) {
			self.multimap.iter().for_each(|(key, values)| {
				let mut set = BoundedBTreeSet::<T::Value, T::CapacityPerKey>::new();
				values.iter().for_each(|value| {
					Index::<T, I>::insert(value, key);
					let _ = set.try_insert(value.clone());
				});
				Map::<T, I>::insert(key.clone(), set);
			});
		}
	}
}
