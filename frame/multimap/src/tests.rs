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

use crate::{mock::*, traits::UniqueMultimap, Error};
use std::collections::BTreeSet;

#[test]
fn unique_multimap_works() {
	new_test_ext().execute_with(|| {
		assert!(matches!(Multimap::try_insert(0, 1000), Ok(true)));
		assert_eq!(Multimap::get(0), BTreeSet::from_iter(vec![1000]));

		assert!(matches!(Multimap::try_insert(0, 1001), Ok(true)));
		assert_eq!(Multimap::get(0), BTreeSet::from_iter(vec![1000, 1001]));

		// overflow capacity per key
		assert!(matches!(Multimap::try_insert(0, 1002), Err(Error::<Test>::CapacityOverflow)));
		assert!(matches!(Multimap::try_insert(0, 1001), Ok(false)));
		assert_eq!(Multimap::get(0), BTreeSet::from_iter(vec![1000, 1001]));

		// duplicate value
		assert!(matches!(Multimap::try_insert(1, 1000), Err(Error::<Test>::DuplicateValue)));

		assert!(Multimap::remove_all(0));
	});
}
