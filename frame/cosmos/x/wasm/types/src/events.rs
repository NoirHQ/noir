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

pub const EVENT_TYPE_STORE_CODE: &str = "store_code";
pub const EVENT_TYPE_INSTANTIATE: &str = "instantiate";
pub const EVENT_TYPE_EXECUTE: &str = "execute";
pub const EVENT_TYPE_MIGRATE: &str = "migrate";
pub const EVENT_TYPE_UPDATE_CONTRACT_ADMIN: &str = "update_contract_admin";

pub const ATTRIBUTE_KEY_CONTRACT_ADDR: &str = "_contract_address";
pub const ATTRIBUTE_KEY_CODE_ID: &str = "code_id";
pub const ATTRIBUTE_KEY_CHECKSUM: &str = "code_checksum";
pub const ATTRIBUTE_KEY_NEW_ADMIN: &str = "new_admin_address";
