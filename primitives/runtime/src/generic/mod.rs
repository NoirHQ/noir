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

mod unchecked_extrinsic;

pub use unchecked_extrinsic::*;

use frame_support::{
	dispatch::{DispatchInfo, GetDispatchInfo},
	traits::ExtrinsicCall,
};
use scale_info::TypeInfo;
use sp_runtime::traits::{Dispatchable, TransactionExtension};

impl<Address, Call, Signature, Extension> GetDispatchInfo
	for UncheckedExtrinsic<Address, Call, Signature, Extension>
where
	Call: GetDispatchInfo + Dispatchable,
	Extension: TransactionExtension<Call>,
{
	fn get_dispatch_info(&self) -> DispatchInfo {
		let mut info = self.function.get_dispatch_info();
		info.extension_weight = self.extension_weight();
		info
	}
}

impl<Address, Call, Signature, Extension> ExtrinsicCall
	for UncheckedExtrinsic<Address, Call, Signature, Extension>
where
	Address: TypeInfo,
	Call: TypeInfo,
	Signature: TypeInfo,
	Extension: TypeInfo,
{
	type Call = Call;

	fn call(&self) -> &Self::Call {
		&self.function
	}
}
