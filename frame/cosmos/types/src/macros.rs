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

#[macro_export]
macro_rules! any_match {
	($msg:expr, { $( $msg_type:ty => $handler:expr ),* $(,)? }, $default:expr) => {
		{
			{
				$(
					if $msg.type_url == <$msg_type as cosmos_sdk_proto::traits::Name>::type_url() {
                        $handler
					} else
				)* {
                    $default
				}
			}
        }
	};
}

#[cfg(test)]
mod tests {
	use cosmos_sdk_proto::{
		cosmos::bank::v1beta1::MsgSend,
		cosmwasm::wasm::v1::{MsgExecuteContract, MsgStoreCode},
		prost::Name,
		traits::Message,
		Any,
	};

	#[test]
	fn any_match_test() {
		let any = Any { type_url: MsgSend::type_url(), value: MsgSend::default().encode_to_vec() };
		let result = any_match!(
			any, {
				MsgSend => any.type_url,
				MsgStoreCode => any.type_url,
			},
			"Unsupported msg".to_string()
		);
		assert_eq!(result, MsgSend::type_url());

		let any = Any {
			type_url: MsgExecuteContract::type_url(),
			value: MsgExecuteContract::default().encode_to_vec(),
		};
		let result = any_match!(
			any, {
				MsgSend => any.type_url,
				MsgStoreCode => any.type_url,
			},
			"Unsupported msg".to_string()
		);
		assert_eq!(result, "Unsupported msg".to_string());
	}
}
