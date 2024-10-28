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

use crate as frame_babel;
use cosmos_sdk_proto::{
	cosmos::bank::v1beta1::MsgSend,
	cosmwasm::wasm::v1::{
		MsgExecuteContract, MsgInstantiateContract2, MsgMigrateContract, MsgStoreCode,
		MsgUpdateAdmin,
	},
	Any,
};
use frame_babel::{
	cosmos::{
		address::{AccountToAddr, AddressMapping as CosmosAddressMapping},
		precompile::Precompiles,
	},
	ethereum::{AddressMapping as EthereumAddressMapping, EnsureAddress},
	extensions::unify_account,
	VarAddress,
};
use frame_support::{
	derive_impl,
	instances::{Instance1, Instance2},
	parameter_types,
	traits::{ConstU32, Contains, NeverEnsureOrigin},
	PalletId,
};
use frame_system::EnsureRoot;
use np_cosmos::traits::CosmosHub;
use np_runtime::{AccountId32, MultiSigner};
use pallet_cosmos::{
	config_preludes::{MaxDenomLimit, NativeAssetId},
	types::DenomOf,
};
use pallet_cosmos_types::{
	any_match,
	coin::DecCoin,
	context::{self, Context},
	msgservice,
};
use pallet_cosmos_x_auth::AnteDecorators;
use pallet_cosmos_x_auth_signing::{
	sign_mode_handler::SignModeHandler, sign_verifiable_tx::SigVerifiableTx,
};
use pallet_cosmos_x_bank::MsgSendHandler;
use pallet_cosmos_x_wasm::msgs::{
	MsgExecuteContractHandler, MsgInstantiateContract2Handler, MsgMigrateContractHandler,
	MsgStoreCodeHandler, MsgUpdateAdminHandler,
};
use pallet_cosmwasm::instrument::CostRules;
use pallet_multimap::traits::UniqueMap;
use sp_core::{ConstU128, Pair, H256};
use sp_runtime::{
	traits::{IdentityLookup, TryConvert},
	BoundedVec, BuildStorage,
};

pub type AccountId = AccountId32<MultiSigner>;
pub type Balance = u128;
pub type AssetId = u32;
pub type Hash = H256;

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(10)]
	pub type Balances = pallet_balances;
	#[runtime::pallet_index(11)]
	pub type Assets = pallet_assets;

	#[runtime::pallet_index(20)]
	pub type Sudo = pallet_sudo;

	#[runtime::pallet_index(30)]
	pub type Ethereum = pallet_ethereum;
	#[runtime::pallet_index(31)]
	pub type EVM = pallet_evm;

	#[runtime::pallet_index(40)]
	pub type Cosmos = pallet_cosmos;
	#[runtime::pallet_index(41)]
	pub type Cosmwasm = pallet_cosmwasm;

	#[runtime::pallet_index(50)]
	pub type AddressMap = pallet_multimap<Instance1>;
	#[runtime::pallet_index(51)]
	pub type AssetMap = pallet_multimap<Instance2>;
	#[runtime::pallet_index(100)]
	pub type Babel = frame_babel;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId;
	type Block = frame_system::mocking::MockBlock<Self>;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Test {}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}
#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type AccountStore = System;
	type ExistentialDeposit = ExistentialDeposit;
}

#[derive_impl(pallet_assets::config_preludes::TestDefaultConfig)]
impl pallet_assets::Config for Test {
	type Balance = Balance;
	type AssetId = AssetId;
	type Currency = Balances;
	type CreateOrigin = NeverEnsureOrigin<AccountId>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type Freezer = ();
	type AssetDeposit = ConstU128<500>;
	type AssetAccountDeposit = ConstU128<500>;
	type MetadataDepositBase = ConstU128<0>;
	type MetadataDepositPerByte = ConstU128<0>;
	type ApprovalDeposit = ConstU128<0>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

#[derive_impl(pallet_sudo::config_preludes::TestDefaultConfig)]
impl pallet_sudo::Config for Test {}

#[derive_impl(pallet_ethereum::config_preludes::TestDefaultConfig)]
impl pallet_ethereum::Config for Test {}

#[derive_impl(pallet_evm::config_preludes::TestDefaultConfig)]
impl pallet_evm::Config for Test {
	type CallOrigin = EnsureAddress<AccountId>;
	type WithdrawOrigin = EnsureAddress<AccountId>;
	type AddressMapping = EthereumAddressMapping<Self>;
	type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
	type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
	type Currency = Balances;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type Timestamp = Timestamp;
}

parameter_types! {
	pub const NativeDenom: &'static str = "antt";
}

pub struct MinGasPrices;
impl context::traits::MinGasPrices for MinGasPrices {
	fn min_prices() -> Vec<DecCoin> {
		vec![DecCoin { denom: NativeDenom::get().to_string(), amount: 1 }]
	}
}

pub struct AssetToDenom;
impl TryConvert<String, AssetId> for AssetToDenom {
	fn try_convert(denom: String) -> Result<AssetId, String> {
		if denom == NativeDenom::get() {
			Ok(NativeAssetId::get())
		} else {
			let denom_raw: BoundedVec<u8, MaxDenomLimit> =
				denom.as_bytes().to_vec().try_into().map_err(|_| denom.clone())?;
			AssetMap::find_key(denom_raw).ok_or(denom.clone())
		}
	}
}
impl TryConvert<AssetId, String> for AssetToDenom {
	fn try_convert(asset_id: AssetId) -> Result<String, AssetId> {
		if asset_id == NativeAssetId::get() {
			Ok(NativeDenom::get().to_string())
		} else {
			let denom =
				<AssetMap as UniqueMap<AssetId, BoundedVec<u8, MaxDenomLimit>>>::get(asset_id)
					.ok_or(asset_id)?;
			String::from_utf8(denom.into()).map_err(|_| asset_id)
		}
	}
}

pub struct MsgFilter;
impl Contains<Any> for MsgFilter {
	fn contains(msg: &Any) -> bool {
		any_match!(
			msg, {
				MsgSend => true,
				MsgStoreCode => true,
				MsgInstantiateContract2 => true,
				MsgExecuteContract => true,
				MsgMigrateContract => true,
				MsgUpdateAdmin => true,
			},
			false
		)
	}
}

pub struct MsgServiceRouter;
impl msgservice::traits::MsgServiceRouter<Context> for MsgServiceRouter {
	fn route(msg: &Any) -> Option<Box<dyn msgservice::traits::MsgHandler<Context>>> {
		any_match!(
			msg, {
				MsgSend => Some(Box::<MsgSendHandler<Test>>::default()),
				MsgStoreCode => Some(Box::<MsgStoreCodeHandler<Test>>::default()),
				MsgInstantiateContract2 => Some(Box::<MsgInstantiateContract2Handler<Test>>::default()),
				MsgExecuteContract => Some(Box::<MsgExecuteContractHandler<Test>>::default()),
				MsgMigrateContract => Some(Box::<MsgMigrateContractHandler<Test>>::default()),
				MsgUpdateAdmin => Some(Box::<MsgUpdateAdminHandler<Test>>::default()),
			},
			None
		)
	}
}

#[derive_impl(pallet_cosmos::config_preludes::TestDefaultConfig)]
impl pallet_cosmos::Config for Test {
	type AddressMapping = CosmosAddressMapping<Self>;
	type Balance = Balance;
	type AssetId = AssetId;
	type NativeAsset = Balances;
	type Assets = Assets;
	type NativeDenom = NativeDenom;
	type MinGasPrices = MinGasPrices;
	type AssetToDenom = AssetToDenom;
	type Context = Context;
	type ChainInfo = CosmosHub;
	type AnteHandler = AnteDecorators<Self>;
	type MsgFilter = MsgFilter;
	type MsgServiceRouter = MsgServiceRouter;
	type SigVerifiableTx = SigVerifiableTx;
	type SignModeHandler = SignModeHandler;
}

parameter_types! {
	pub const CosmwasmPalletId: PalletId = PalletId(*b"cosmwasm");
	pub const MaxContractLabelSize: u32 = 64;
	pub const MaxContractTrieIdSize: u32 = Hash::len_bytes() as u32;
	pub const MaxInstantiateSaltSize: u32 = 128;
	pub const MaxFundsAssets: u32 = 32;
	pub const CodeTableSizeLimit: u32 = 4096;
	pub const CodeGlobalVariableLimit: u32 = 256;
	pub const CodeParameterLimit: u32 = 128;
	pub const CodeBranchTableSizeLimit: u32 = 256;
	pub const CodeStorageByteDeposit: u32 = 1_000_000;
	pub const ContractStorageByteReadPrice: u32 = 1;
	pub const ContractStorageByteWritePrice: u32 = 1;
	pub WasmCostRules: CostRules<Test> = Default::default();
}

impl pallet_cosmwasm::Config for Test {
	const MAX_FRAMES: u8 = 64;
	type RuntimeEvent = RuntimeEvent;
	type AccountIdExtended = AccountId;
	type PalletId = CosmwasmPalletId;
	type MaxCodeSize = ConstU32<{ 1024 * 1024 }>;
	type MaxInstrumentedCodeSize = ConstU32<{ 2 * 1024 * 1024 }>;
	type MaxMessageSize = ConstU32<{ 64 * 1024 }>;
	type AccountToAddr = AccountToAddr<Self>;
	type AssetToDenom = AssetToDenom;
	type Balance = Balance;
	type AssetId = AssetId;
	type Assets = Assets;
	type NativeAsset = Balances;
	type ChainInfo = CosmosHub;
	type MaxContractLabelSize = MaxContractLabelSize;
	type MaxContractTrieIdSize = MaxContractTrieIdSize;
	type MaxInstantiateSaltSize = MaxInstantiateSaltSize;
	type MaxFundsAssets = MaxFundsAssets;

	type CodeTableSizeLimit = CodeTableSizeLimit;
	type CodeGlobalVariableLimit = CodeGlobalVariableLimit;
	type CodeStackLimit = ConstU32<{ u32::MAX }>;

	type CodeParameterLimit = CodeParameterLimit;
	type CodeBranchTableSizeLimit = CodeBranchTableSizeLimit;
	type CodeStorageByteDeposit = CodeStorageByteDeposit;
	type ContractStorageByteReadPrice = ContractStorageByteReadPrice;
	type ContractStorageByteWritePrice = ContractStorageByteWritePrice;

	type WasmCostRules = WasmCostRules;
	type UnixTime = Timestamp;
	type WeightInfo = pallet_cosmwasm::weights::SubstrateWeight<Test>;

	type PalletHook = Precompiles<Test>;

	type UploadWasmOrigin = frame_system::EnsureSigned<Self::AccountId>;

	type ExecuteWasmOrigin = frame_system::EnsureSigned<Self::AccountId>;

	type NativeAssetId = NativeAssetId;

	type NativeDenom = NativeDenom;
}

#[derive_impl(pallet_multimap::config_preludes::TestDefaultConfig)]
impl pallet_multimap::Config<Instance1> for Test {
	type Key = AccountId;
	type Value = VarAddress;
	type CapacityPerKey = ConstU32<{ VarAddress::variant_count() }>;
}

#[derive_impl(pallet_multimap::config_preludes::TestDefaultConfig)]
impl pallet_multimap::Config<Instance2> for Test {
	type Key = AssetId;
	type Value = DenomOf<Self>;
}

impl frame_babel::Config for Test {
	type AddressMap = AddressMap;
	type AssetMap = AssetMap;
	type Balance = Balance;
}

impl unify_account::Config for Test {
	type AddressMap = AddressMap;
	type DrainBalance = Balances;
}

pub fn alice() -> AccountId {
	sp_keyring::sr25519::Keyring::Alice.pair().public().into()
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> { balances: vec![(alice(), 10000)] }
		.assimilate_storage(&mut t)
		.unwrap();
	t.into()
}
