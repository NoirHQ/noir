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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate parity_scale_codec as codec;

#[doc(hidden)]
pub mod __private {
	pub use sp_core;
	pub use sp_runtime;
}

mod accountid32;
pub mod generic;
mod multi;
pub mod self_contained;
pub mod traits;

pub use accountid32::AccountId32;
pub use multi::{MultiSignature, MultiSigner};
