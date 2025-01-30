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

pub const EVENT_TYPE_STORE_CODE: &str = "store_code";
pub const EVENT_TYPE_INSTANTIATE: &str = "instantiate";
pub const EVENT_TYPE_EXECUTE: &str = "execute";
pub const EVENT_TYPE_MIGRATE: &str = "migrate";
pub const EVENT_TYPE_UPDATE_CONTRACT_ADMIN: &str = "update_contract_admin";

pub const ATTRIBUTE_KEY_CONTRACT_ADDR: &str = "_contract_address";
pub const ATTRIBUTE_KEY_CODE_ID: &str = "code_id";
pub const ATTRIBUTE_KEY_CHECKSUM: &str = "code_checksum";
pub const ATTRIBUTE_KEY_NEW_ADMIN: &str = "new_admin_address";
