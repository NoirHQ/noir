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

pub use np_multimap::traits::*;

use alloc::collections::BTreeSet;
use frame_support::ensure;
use parity_scale_codec::EncodeLike;

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
		if let Some(value) = Map::<T, I>::take(key).first() {
			Index::<T, I>::remove(value);
		}
	}
}
