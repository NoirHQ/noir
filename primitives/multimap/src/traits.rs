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

use alloc::collections::BTreeSet;
use parity_scale_codec::{Codec, EncodeLike, FullCodec};

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

/// Helper function to transmute [`EncodeLike`] to expected type.
pub fn transmute<T: EncodeLike<R>, R: Codec>(value: T) -> R {
	value.using_encoded(|encoded| R::decode(&mut &encoded[..]).expect("Decoding failed"))
}
