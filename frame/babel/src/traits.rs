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

use core::fmt::Debug;
use frame_support::{
	sp_runtime::traits::{MaybeDisplay, MaybeSerializeDeserialize, Member},
	Parameter,
};
use parity_scale_codec::MaxEncodedLen;

/// Trait for providing the account id type.
pub trait AccountIdProvider {
	type AccountId: Parameter
		+ Member
		+ MaybeSerializeDeserialize
		+ Debug
		+ MaybeDisplay
		+ Ord
		+ MaxEncodedLen;
}

impl<T: frame_system::Config> AccountIdProvider for T {
	type AccountId = T::AccountId;
}
