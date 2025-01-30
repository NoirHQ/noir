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

use crate::traits;
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

impl<K: FullCodec, V: FullCodec + Ord> traits::UniqueMultimap<K, V> for UniqueMultimap<K, V> {
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

	fn remove<KeyArg: EncodeLike<K>, ValArg: EncodeLike<V>>(key: KeyArg, value: ValArg) -> bool {
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

#[cfg(test)]
mod tests {
	use crate::traits::UniqueMultimap;
	use std::collections::BTreeSet;

	type Multimap = super::UniqueMultimap<String, u32>;

	#[test]
	fn unique_multimap_insert() {
		assert_eq!(Multimap::try_insert("alice".to_string(), 1), Ok(true));
		assert_eq!(Multimap::try_insert("alice".to_string(), 1), Ok(false));
		assert_eq!(Multimap::try_insert("bob".to_string(), 1), Err("Duplicate value"));
		assert_eq!(Multimap::try_insert("alice".to_string(), 2), Ok(true));
		assert_eq!(Multimap::try_insert("bob".to_string(), 3), Ok(true));
		assert_eq!(Multimap::try_insert("bob".to_string(), 1), Err("Duplicate value"));
	}

	#[test]
	fn unique_multimap_get() {
		let _ = Multimap::try_insert("alice".to_string(), 1);
		let _ = Multimap::try_insert("alice".to_string(), 2);
		let _ = Multimap::try_insert("bob".to_string(), 3);
		assert_eq!(Multimap::get("alice"), BTreeSet::from([1, 2]));
		assert_eq!(Multimap::find_key(1), Some("alice".to_string()));
		assert_eq!(Multimap::find_key(3), Some("bob".to_string()));
	}

	#[test]
	fn unique_multimap_remove() {
		let _ = Multimap::try_insert("alice".to_string(), 1);
		let _ = Multimap::try_insert("alice".to_string(), 2);
		let _ = Multimap::try_insert("alice".to_string(), 3);
		assert!(Multimap::remove("alice", 2));
		assert_eq!(Multimap::get("alice"), BTreeSet::from([1, 3]));
		assert!(!Multimap::remove("alice", 2));
		assert!(Multimap::remove_all("alice"));
		assert!(Multimap::get("alice").is_empty());
		assert!(!Multimap::remove_all("alice"));
	}
}
