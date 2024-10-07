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

pub mod cosmos;

use jsonrpsee::{
	core::to_json_raw_value,
	types::{
		error::{INTERNAL_ERROR_CODE, INVALID_REQUEST_CODE},
		ErrorObject, ErrorObjectOwned,
	},
};

pub fn error<T: ToString>(code: i32, message: T, data: Option<&[u8]>) -> ErrorObjectOwned {
	ErrorObject::owned(
		code,
		message.to_string(),
		data.map(|bytes| {
			to_json_raw_value(&format!("0x{}", hex::encode(bytes)))
				.expect("Failed to serialize data")
		}),
	)
}

pub fn request_error<T: ToString>(message: T) -> ErrorObjectOwned {
	error(INVALID_REQUEST_CODE, message, None)
}

pub fn internal_error<T: ToString>(message: T) -> ErrorObjectOwned {
	error(INTERNAL_ERROR_CODE, message, None)
}
