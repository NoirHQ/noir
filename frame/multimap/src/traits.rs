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

use crate::*;

use alloc::collections::BTreeSet;
use frame_support::ensure;
use parity_scale_codec::{Codec, EncodeLike, FullCodec};

fn transmute<T: EncodeLike<R>, R: Codec>(value: T) -> R {
	value.using_encoded(|encoded| R::decode(&mut &encoded[..]).expect("Decoding failed"))
}

/// Unique multimap whose values are unique across all keys.
pub trait UniqueMultimap<K: FullCodec, V: FullCodec> {
	type Error;

	/// Tries to insert a value into the multimap.
	fn try_insert<KeyArg: EncodeLike<K>, ValArg: EncodeLike<V>>(
		key: KeyArg,
		value: ValArg,
	) -> Result<bool, Self::Error>;

	/// Gets all values for a key.
	fn get<KeyArg: EncodeLike<K>>(key: KeyArg) -> BTreeSet<V>;

	/// Finds the key for a value.
	fn find_key<ValArg: EncodeLike<V>>(value: ValArg) -> Option<K>;

	/// Removes a value from the multimap.
	fn remove<KeyArg: EncodeLike<K>, ValArg: EncodeLike<V>>(key: KeyArg, value: ValArg) -> bool;

	/// Removes all values for a key.
	fn remove_all<KeyArg: EncodeLike<K>>(key: KeyArg) -> bool;
}

impl<T: Config<I>, I: 'static> UniqueMultimap<T::Key, T::Value> for Pallet<T, I> {
	type Error = Error<T, I>;

	fn try_insert<K: EncodeLike<T::Key>, V: EncodeLike<T::Value>>(
		key: K,
		value: V,
	) -> Result<bool, Error<T, I>> {
		let key = transmute(key);
		let value = transmute(value);
		Map::<T, I>::try_mutate(&key, |values| {
			ensure!(
				Index::<T, I>::get(&value).filter(|k| *k != key).is_none(),
				Error::<T, I>::DuplicateValue
			);
			values
				.try_insert(value.clone())
				.inspect(|ok| {
					if *ok {
						Index::<T, I>::insert(value, key.clone());
					}
				})
				.map_err(|_| Error::<T, I>::CapacityOverflow)
		})
	}

	fn get<K: EncodeLike<T::Key>>(key: K) -> BTreeSet<T::Value> {
		Map::<T, I>::get(key).into()
	}

	fn find_key<V: EncodeLike<T::Value>>(value: V) -> Option<T::Key> {
		Index::<T, I>::get(value)
	}

	fn remove<K: EncodeLike<T::Key>, V: EncodeLike<T::Value>>(key: K, value: V) -> bool {
		let value = transmute(value);
		Map::<T, I>::try_mutate(key, |values| -> Result<bool, ()> {
			Ok(values.remove(&value).then(|| Index::<T, I>::remove(value)).is_some())
		})
		.unwrap_or(false)
	}

	fn remove_all<K: EncodeLike<T::Key>>(key: K) -> bool {
		Map::<T, I>::take(key).into_iter().fold(false, |_, value| {
			Index::<T, I>::remove(value);
			true
		})
	}
}

/// Unique map whose the value is unique across all keys.
pub trait UniqueMap<K: FullCodec, V: FullCodec> {
	type Error;

	/// Tries to insert a value into the map.
	fn try_insert<KeyArg: EncodeLike<K>, ValArg: EncodeLike<V>>(
		key: KeyArg,
		value: ValArg,
	) -> Result<bool, Self::Error>;

	/// Gets the value for a key.
	fn get<KeyArg: EncodeLike<K>>(key: KeyArg) -> Option<V>;

	/// Finds the key for a value.
	fn find_key<ValArg: EncodeLike<V>>(value: ValArg) -> Option<K>;

	/// Removes a value from the map.
	fn remove<KeyArg: EncodeLike<K>>(key: KeyArg);
}

impl<T: Config<I>, I: 'static> UniqueMap<T::Key, T::Value> for Pallet<T, I> {
	type Error = Error<T, I>;

	fn try_insert<K: EncodeLike<T::Key>, V: EncodeLike<T::Value>>(
		key: K,
		value: V,
	) -> Result<bool, Error<T, I>> {
		let key = transmute(key);
		let value = transmute(value);
		Map::<T, I>::try_mutate(&key, |values| {
			ensure!(
				Index::<T, I>::get(&value).filter(|k| *k != key).is_none(),
				Error::<T, I>::DuplicateValue
			);

			*values = BTreeSet::from([value.clone()])
				.try_into()
				.map_err(|_| Error::<T, I>::CapacityOverflow)?;
			Index::<T, I>::insert(value, key.clone());

			Ok(true)
		})
	}

	fn get<K: EncodeLike<T::Key>>(key: K) -> Option<T::Value> {
		Map::<T, I>::get(key).first().cloned()
	}

	fn find_key<V: EncodeLike<T::Value>>(value: V) -> Option<T::Key> {
		Index::<T, I>::get(value)
	}

	fn remove<K: EncodeLike<T::Key>>(key: K) {
		Map::<T, I>::remove(key);
	}
}

#[cfg(feature = "std")]
pub mod in_mem {
	use parity_scale_codec::{EncodeLike, FullCodec};
	use std::{
		cell::RefCell,
		collections::{BTreeMap, BTreeSet},
		marker::PhantomData,
	};

	thread_local! {
		static UNIQUE_MULTIMAP: RefCell<BTreeMap<Vec<u8>, BTreeSet<Vec<u8>>>> = const { RefCell::new(BTreeMap::new()) };
		static UNIQUE_MULTIMAP_INDEX: RefCell<BTreeMap<Vec<u8>, Vec<u8>>> = const { RefCell::new(BTreeMap::new()) };
	}

	pub struct UniqueMultimap<K, V>(PhantomData<(K, V)>);

	impl<K: FullCodec, V: FullCodec + Ord> super::UniqueMultimap<K, V> for UniqueMultimap<K, V> {
		type Error = &'static str;

		fn try_insert<KeyArg: EncodeLike<K>, ValArg: EncodeLike<V>>(
			key: KeyArg,
			value: ValArg,
		) -> Result<bool, Self::Error> {
			let key = key.encode();
			let value = value.encode();
			match UNIQUE_MULTIMAP_INDEX.with(|index| {
				let mut index = index.borrow_mut();
				if let Some(existing_key) = index.get(&value) {
					if existing_key != &key {
						Err("Duplicate value")
					} else {
						Ok(false)
					}
				} else {
					index.insert(value.clone(), key.clone());
					Ok(true)
				}
			}) {
				Ok(true) => {
					UNIQUE_MULTIMAP.with(|map| {
						let mut map = map.borrow_mut();
						map.entry(key).or_default().insert(value);
					});
					Ok(true)
				},
				Ok(false) => Ok(false),
				Err(e) => Err(e),
			}
		}

		fn get<KeyArg: EncodeLike<K>>(key: KeyArg) -> BTreeSet<V> {
			let key = key.encode();
			UNIQUE_MULTIMAP.with(|map| {
				map.borrow()
					.get(&key)
					.map(|values| {
						values
							.iter()
							.map(|value| V::decode(&mut &value[..]).expect("Decoding failed"))
							.collect()
					})
					.unwrap_or_default()
			})
		}

		fn find_key<ValArg: EncodeLike<V>>(value: ValArg) -> Option<K> {
			let value = value.encode();
			UNIQUE_MULTIMAP_INDEX.with(|index| {
				index
					.borrow()
					.get(&value)
					.map(|key| K::decode(&mut &key[..]).expect("Decoding failed"))
			})
		}

		fn remove<KeyArg: EncodeLike<K>, ValArg: EncodeLike<V>>(
			key: KeyArg,
			value: ValArg,
		) -> bool {
			let key = key.encode();
			let value = value.encode();
			if UNIQUE_MULTIMAP_INDEX.with(|index| {
				let mut index = index.borrow_mut();
				index.remove(&value).is_some()
			}) {
				UNIQUE_MULTIMAP.with(|map| {
					let mut map = map.borrow_mut();
					if let Some(values) = map.get_mut(&key) {
						values.remove(&value);
					}
				});
				true
			} else {
				false
			}
		}

		fn remove_all<KeyArg: EncodeLike<K>>(key: KeyArg) -> bool {
			let key = key.encode();
			if let Some(values) = UNIQUE_MULTIMAP.with(|map| map.borrow_mut().remove(&key)) {
				UNIQUE_MULTIMAP_INDEX.with(|index| {
					let mut index = index.borrow_mut();
					for value in values {
						index.remove(&value);
					}
				});
				true
			} else {
				false
			}
		}
	}
}
