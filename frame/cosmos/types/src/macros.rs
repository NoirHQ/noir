// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

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
		Any,
	};

	#[test]
	fn any_match_test() {
		let any = Any::from_msg(&MsgSend::default()).unwrap();
		let result = any_match!(
			any, {
				MsgSend => any.type_url,
				MsgStoreCode => any.type_url,
			},
			"Unsupported msg".to_string()
		);
		assert_eq!(result, MsgSend::type_url());

		let any = Any::from_msg(&MsgExecuteContract::default()).unwrap();
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
