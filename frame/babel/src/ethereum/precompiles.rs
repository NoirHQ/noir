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

use super::precompile;
use frame_support::parameter_types;
use pallet_evm_precompile_balances_erc20::Erc20BalancesPrecompile;
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use pallet_evm_precompileset_assets_erc20::Erc20AssetsPrecompileSet;
use precompile::Babel;
use precompile_utils::precompile_set::*;

pub const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] =
	&hex_literal::hex!("ffffffff000000000000000000000000");

parameter_types! {
	pub AssetPrefix: &'static [u8] = ASSET_PRECOMPILE_ADDRESS_PREFIX;
}

type EthereumPrecompilesChecks = (AcceptDelegateCall, CallableByContract, CallableByPrecompile);

#[precompile_utils::precompile_name_from_address]
type BabelPrecompilesAt<T> = (
	PrecompileAt<AddressU64<1>, ECRecover, EthereumPrecompilesChecks>,
	PrecompileAt<AddressU64<2>, Sha256, EthereumPrecompilesChecks>,
	PrecompileAt<AddressU64<3>, Ripemd160, EthereumPrecompilesChecks>,
	PrecompileAt<AddressU64<4>, Identity, EthereumPrecompilesChecks>,
	PrecompileAt<AddressU64<5>, Modexp, EthereumPrecompilesChecks>,
	PrecompileAt<AddressU64<6>, Bn128Add, EthereumPrecompilesChecks>,
	PrecompileAt<AddressU64<7>, Bn128Mul, EthereumPrecompilesChecks>,
	PrecompileAt<AddressU64<8>, Bn128Pairing, EthereumPrecompilesChecks>,
	PrecompileAt<AddressU64<9>, Blake2F, EthereumPrecompilesChecks>,
	PrecompileAt<AddressU64<0x400>, Babel<T>, (CallableByContract, CallableByPrecompile)>,
	PrecompileAt<
		AddressU64<0x401>,
		Erc20BalancesPrecompile<T>,
		(CallableByContract, CallableByPrecompile),
	>,
);

pub type BabelPrecompiles<T> = PrecompileSetBuilder<
	T,
	(
		PrecompilesInRangeInclusive<(AddressU64<1>, AddressU64<2048>), BabelPrecompilesAt<T>>,
		PrecompileSetStartingWith<
			AssetPrefix,
			Erc20AssetsPrecompileSet<T>,
			(CallableByContract, CallableByPrecompile),
		>,
	),
>;
