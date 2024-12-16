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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiDataSliceConfig {
    pub offset: usize,
    pub length: usize,
}

pub fn slice_data(data: &[u8], data_slice_config: Option<UiDataSliceConfig>) -> &[u8] {
    if let Some(UiDataSliceConfig { offset, length }) = data_slice_config {
        if offset >= data.len() {
            &[]
        } else if length > data.len() - offset {
            &data[offset..]
        } else {
            &data[offset..offset + length]
        }
    } else {
        data
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_slice_data() {
        let data = vec![1, 2, 3, 4, 5];
        let slice_config = Some(UiDataSliceConfig {
            offset: 0,
            length: 5,
        });
        assert_eq!(slice_data(&data, slice_config), &data[..]);

        let slice_config = Some(UiDataSliceConfig {
            offset: 0,
            length: 10,
        });
        assert_eq!(slice_data(&data, slice_config), &data[..]);

        let slice_config = Some(UiDataSliceConfig {
            offset: 1,
            length: 2,
        });
        assert_eq!(slice_data(&data, slice_config), &data[1..3]);

        let slice_config = Some(UiDataSliceConfig {
            offset: 10,
            length: 2,
        });
        assert_eq!(slice_data(&data, slice_config), &[] as &[u8]);
    }
}
