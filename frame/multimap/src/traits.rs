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

/// Unique multimap whose values are unique across all keys.
pub trait UniqueMultimap<K, V> {
	type Error;

	/// Tries to insert a value into the multimap.
	fn try_insert(key: K, value: V) -> Result<bool, Self::Error>;

	/// Gets all values for a key.
	fn get(key: K) -> BTreeSet<V>;

	/// Finds the key for a value.
	fn find_key(value: V) -> Option<K>;

	/// Removes a value from the multimap.
	fn remove(key: K, value: V) -> bool;

	/// Removes all values for a key.
	fn remove_all(key: K) -> bool;
}

impl<T: Config<I>, I: 'static> UniqueMultimap<T::Key, T::Value> for Pallet<T, I> {
	type Error = Error<T, I>;

	fn try_insert(key: T::Key, value: T::Value) -> Result<bool, Error<T, I>> {
		Map::<T, I>::try_mutate(&key, |values| {
			ensure!(
				Index::<T, I>::get(&value).filter(|k| *k != key).is_none(),
				Error::<T, I>::DuplicateValue
			);
			values
				.try_insert(value.clone())
				.inspect(|ok| {
					if *ok {
						Index::<T, I>::insert(&value, &key);
					}
				})
				.map_err(|_| Error::<T, I>::CapacityOverflow)
		})
	}

	fn get(key: T::Key) -> BTreeSet<T::Value> {
		Map::<T, I>::get(&key).into()
	}

	fn find_key(value: T::Value) -> Option<T::Key> {
		Index::<T, I>::get(&value)
	}

	fn remove(key: T::Key, value: T::Value) -> bool {
		Map::<T, I>::try_mutate(&key, |values| -> Result<bool, ()> {
			Ok(values.remove(&value).then(|| Index::<T, I>::remove(&value)).is_some())
		})
		.unwrap_or(false)
	}

	fn remove_all(key: T::Key) -> bool {
		Map::<T, I>::take(&key).into_iter().fold(false, |_, value| {
			Index::<T, I>::remove(&value);
			true
		})
	}
}
