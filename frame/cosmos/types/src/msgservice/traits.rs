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

use crate::errors::CosmosError;
use cosmos_sdk_proto::Any;
use nostd::boxed::Box;

pub trait MsgHandler<Context> {
	fn handle(&self, ctx: &mut Context, msg: &Any) -> Result<(), CosmosError>;
}

pub trait MsgServiceRouter<Context> {
	fn route(msg: &Any) -> Option<Box<dyn MsgHandler<Context>>>;
}
