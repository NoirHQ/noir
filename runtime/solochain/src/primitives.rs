// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
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

use crate::*;

pub use noir_core_primitives::*;

use np_runtime::self_contained;
use sp_runtime::{generic, MultiAddress};

pub type Address = MultiAddress<AccountId, AccountIndex>;

pub type Block = generic::Block<Header, UncheckedExtrinsic>;

pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

pub type Migrations = ();

pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckMortality<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_babel::UnifyAccount<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

pub type UncheckedExtrinsic =
	self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;

pub mod opaque {
	use crate::{Aura, Grandpa};
	use alloc::vec::Vec;
	use sp_runtime::impl_opaque_keys;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}
