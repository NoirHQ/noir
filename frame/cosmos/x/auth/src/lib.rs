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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod basic;
pub mod fee;
pub mod msg;
pub mod sigverify;

pub type AnteDecorators<T> = (
	basic::ValidateBasicDecorator<T>,
	basic::TxTimeoutHeightDecorator<T>,
	basic::ValidateMemoDecorator<T>,
	sigverify::ValidateSigCountDecorator<T>,
	msg::KnownMsgDecorator<T>,
	sigverify::SigVerificationDecorator<T>,
	fee::DeductFeeDecorator<T>,
	sigverify::IncrementSequenceDecorator<T>,
);
