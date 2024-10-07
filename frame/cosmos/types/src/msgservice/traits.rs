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

use crate::errors::CosmosError;
use alloc::boxed::Box;
use cosmos_sdk_proto::Any;

pub trait MsgHandler<Context> {
	fn handle(&self, ctx: &mut Context, msg: &Any) -> Result<(), CosmosError>;
}

pub trait MsgServiceRouter<Context> {
	fn route(msg: &Any) -> Option<Box<dyn MsgHandler<Context>>>;
}
