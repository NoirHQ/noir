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

//! Primitive types for implementing multimaps.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use error::Error;
#[cfg(feature = "std")]
pub mod in_mem;
pub mod traits;

mod error {
	/// Error types for the multimap.
	#[derive(Debug)]
	pub enum Error {
		/// The value is already in the multimap.
		DuplicateValue,
		/// The multimap is full.
		CapacityOverflow,
	}
}

use alloc::collections::BTreeSet;
use frame_support::{ensure, traits::Get, BoundedBTreeSet, StorageMap};
use parity_scale_codec::{EncodeLike, FullCodec};
use traits::transmute;

pub struct UniqueMultimapAdapter<
	K: FullCodec + Clone + PartialEq,
	V: FullCodec + Clone + Ord,
	Map: StorageMap<K, BoundedBTreeSet<V, Capacity>, Query = BoundedBTreeSet<V, Capacity>>,
	Index: StorageMap<V, K, Query = Option<K>>,
	Capacity: Get<u32>,
	E: From<Error>,
>(core::marker::PhantomData<(K, V, Map, Index, Capacity, E)>);

impl<K, V, Map, Index, Capacity, E> traits::UniqueMultimap<K, V>
	for UniqueMultimapAdapter<K, V, Map, Index, Capacity, E>
where
	K: FullCodec + Clone + PartialEq,
	V: FullCodec + Clone + Ord,
	Map: StorageMap<K, BoundedBTreeSet<V, Capacity>, Query = BoundedBTreeSet<V, Capacity>>,
	Index: StorageMap<V, K, Query = Option<K>>,
	Capacity: Get<u32>,
	E: From<Error>,
{
	type Error = E;

	fn try_insert<KeyArg: EncodeLike<K>, ValArg: EncodeLike<V>>(
		key: KeyArg,
		value: ValArg,
	) -> Result<bool, Self::Error> {
		let key = transmute(key);
		let value = transmute(value);
		Map::try_mutate(&key, |values| {
			ensure!(Index::get(&value).filter(|k| *k != key).is_none(), Error::DuplicateValue);
			values
				.try_insert(value.clone())
				.inspect(|ok| {
					if *ok {
						Index::insert(value, key.clone());
					}
				})
				.map_err(|_| Error::CapacityOverflow.into())
		})
	}

	fn get<KeyArg: EncodeLike<K>>(key: KeyArg) -> BTreeSet<V> {
		Map::get(key).into()
	}

	fn find_key<ValArg: EncodeLike<V>>(value: ValArg) -> Option<K> {
		Index::get(value)
	}

	fn remove<KeyArg: EncodeLike<K>, ValArg: EncodeLike<V>>(key: KeyArg, value: ValArg) -> bool {
		let value = transmute(value);
		Map::try_mutate(key, |values| -> Result<bool, ()> {
			Ok(values.remove(&value).then(|| Index::remove(value)).is_some())
		})
		.unwrap_or(false)
	}

	fn remove_all<KeyArg: EncodeLike<K>>(key: KeyArg) -> bool {
		Map::take(key).into_iter().fold(false, |_, value| {
			Index::remove(value);
			true
		})
	}
}

pub struct UniqueMapAdapter<
	K: FullCodec + Clone + PartialEq,
	V: FullCodec + Clone + Ord,
	Map: StorageMap<K, V, Query = Option<V>>,
	Index: StorageMap<V, K, Query = Option<K>>,
	E: From<Error>,
>(core::marker::PhantomData<(K, V, Map, Index, E)>);

impl<K, V, Map, Index, E> traits::UniqueMap<K, V> for UniqueMapAdapter<K, V, Map, Index, E>
where
	K: FullCodec + Clone + PartialEq,
	V: FullCodec + Clone + Ord,
	Map: StorageMap<K, V, Query = Option<V>>,
	Index: StorageMap<V, K, Query = Option<K>>,
	E: From<Error>,
{
	type Error = E;

	fn try_insert<KeyArg: EncodeLike<K>, ValArg: EncodeLike<V>>(
		key: KeyArg,
		value: ValArg,
	) -> Result<bool, Self::Error> {
		let key = transmute(key);
		let value = transmute(value);
		Map::try_mutate(key.clone(), |v| {
			ensure!(Index::get(&value).filter(|k| *k != key).is_none(), Error::DuplicateValue);

			*v = Some(value.clone());
			Index::insert(value, key);

			Ok(true)
		})
	}

	fn get<KeyArg: EncodeLike<K>>(key: KeyArg) -> Option<V> {
		Map::get(key)
	}

	fn find_key<ValArg: EncodeLike<V>>(value: ValArg) -> Option<K> {
		Index::get(value)
	}

	fn remove<KeyArg: EncodeLike<K>>(key: KeyArg) {
		if let Some(value) = Map::take(key) {
			Index::remove(value);
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		traits::{UniqueMap, UniqueMultimap},
		Error,
	};
	use frame_support::{derive_impl, pallet_prelude::*};
	use sp_io::TestExternalities;
	use std::collections::BTreeSet;

	#[frame_support::pallet]
	pub mod pallet {
		use super::*;

		#[pallet::config]
		pub trait Config: frame_system::Config {}
		#[pallet::pallet]
		pub struct Pallet<T>(_);
		#[pallet::storage]
		pub type MultiMap<T: Config> =
			StorageMap<_, Twox64Concat, u64, BoundedBTreeSet<u32, ConstU32<2>>, ValueQuery>;
		#[pallet::storage]
		pub type Map<T: Config> = StorageMap<_, Twox64Concat, u64, u32>;
		#[pallet::storage]
		pub type Index<T: Config> = StorageMap<_, Twox64Concat, u32, u64>;
	}

	#[frame_support::runtime]
	mod runtime {
		#[runtime::runtime]
		#[runtime::derive(RuntimeCall, RuntimeEvent, RuntimeError, RuntimeOrigin, RuntimeTask)]
		pub struct Test;
		#[runtime::pallet_index(0)]
		pub type System = frame_system;
		#[runtime::pallet_index(1)]
		pub type Pallet = crate::tests::pallet;
	}

	#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
	impl frame_system::Config for Test {
		type Block = frame_system::mocking::MockBlock<Test>;
	}

	impl pallet::Config for Test {}

	pub fn new_test_ext() -> TestExternalities {
		TestExternalities::new(Default::default())
	}

	pub type Multimap = super::UniqueMultimapAdapter<
		u64,
		u32,
		pallet::MultiMap<Test>,
		pallet::Index<Test>,
		ConstU32<2>,
		Error,
	>;

	pub type Map = super::UniqueMapAdapter<u64, u32, pallet::Map<Test>, pallet::Index<Test>, Error>;

	#[test]
	fn unique_multimap_adapter_works() {
		new_test_ext().execute_with(|| {
			assert!(matches!(Multimap::try_insert(0, 1000), Ok(true)));
			assert_eq!(Multimap::get(0), BTreeSet::from_iter(vec![1000]));

			assert!(matches!(Multimap::try_insert(0, 1001), Ok(true)));
			assert_eq!(Multimap::get(0), BTreeSet::from_iter(vec![1000, 1001]));

			// overflow capacity per key
			assert!(matches!(Multimap::try_insert(0, 1002), Err(Error::CapacityOverflow)));
			assert!(matches!(Multimap::try_insert(0, 1001), Ok(false)));
			assert_eq!(Multimap::get(0), BTreeSet::from_iter(vec![1000, 1001]));

			// duplicate value
			assert!(matches!(Multimap::try_insert(1, 1000), Err(Error::DuplicateValue)));

			assert!(Multimap::remove_all(0));
		});
	}

	#[test]
	fn unique_map_adapter_works() {
		new_test_ext().execute_with(|| {
			assert!(matches!(Map::try_insert(0, 1000), Ok(true)));
			assert_eq!(Map::get(0), Some(1000));
			assert_eq!(Map::find_key(1000), Some(0));
			assert_eq!(Map::find_key(1001), None);
			assert!(matches!(Map::try_insert(1, 1000), Err(Error::DuplicateValue)));
			Map::remove(0);
			assert_eq!(Map::get(0), None);
			assert!(matches!(Map::try_insert(1, 1000), Ok(true)));
		});
	}
}
