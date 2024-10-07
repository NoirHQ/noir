// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <http://unlicense.org>

mod xcm_config;

// Substrate and Polkadot dependencies
use alloc::{
	string::{String, ToString},
	vec,
	vec::Vec,
};
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use fp_evm::weight_per_gas;
use frame_babel::{
	cosmos::{self, precompile::Precompiles},
	ethereum::{self, BabelPrecompiles, EnsureAddress, ASSET_PRECOMPILE_ADDRESS_PREFIX},
	extensions::unify_account,
	VarAddress,
};
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	parameter_types,
	traits::{
		ConstBool, ConstU32, ConstU64, ConstU8, EitherOfDiverse, NeverEnsureOrigin,
		TransformOrigin, VariantCountOf,
	},
	weights::{ConstantMultiplier, Weight},
	Blake2_128Concat, PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureSigned,
};
use pallet_assets::pallet::Instance2;
use pallet_cosmos::{
	config_preludes::{
		MaxDenomLimit, MaxMemoCharacters, MsgFilter, NativeAssetId, NativeDenom, TxSigLimit,
		WeightToGas,
	},
	types::{AssetIdOf, DenomOf},
};
use pallet_cosmos_types::{coin::DecCoin, context};
use pallet_cosmos_x_auth_signing::{
	sign_mode_handler::SignModeHandler, sign_verifiable_tx::SigVerifiableTx,
};
use pallet_cosmwasm::instrument::CostRules;
use pallet_ethereum::PostLogContent;
use pallet_multimap::traits::UniqueMap;
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use polkadot_runtime_common::{
	xcm_sender::NoPriceForMessageDelivery, BlockHashCount, SlowAdjustingFeeUpdate,
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{ConstU128, H160, U256};
use sp_runtime::{
	traits::{Convert, TryConvert},
	BoundedVec, Perbill, Permill,
};
use sp_version::RuntimeVersion;
use xcm::latest::prelude::BodyId;

// Local module imports
use super::{
	weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
	AccountId, AddressMap, AssetId, AssetMap, Assets, Aura, Balance, Balances, BaseFee, Block,
	BlockNumber, CollatorSelection, ConsensusHook, Hash, Instance1, MessageQueue, Nonce,
	PalletInfo, ParachainSystem, Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason,
	RuntimeHoldReason, RuntimeOrigin, RuntimeTask, Session, SessionKeys, System, Timestamp,
	WeightToFee, XcmpQueue, AVERAGE_ON_INITIALIZE_RATIO, EXISTENTIAL_DEPOSIT, HOURS,
	MAXIMUM_BLOCK_WEIGHT, MICROUNIT, NORMAL_DISPATCH_RATIO, SLOT_DURATION, VERSION,
};
use xcm_config::{RelayLocation, XcmOriginToTransactDispatchOrigin};

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;

	// This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
	//  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
	// `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
	// the lazy contract deletion.
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u16 = 42;
}

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`ParaChainDefaultConfig`](`struct@frame_system::config_preludes::ParaChainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig)]
impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = sp_runtime::traits::AccountIdLookup<Self::AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The block type.
	type Block = Block;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The action to take on a Runtime Upgrade
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<0>;
	type WeightInfo = ();
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = (CollatorSelection,);
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = 10 * MICROUNIT;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = ConstU8<5>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
}

impl parachain_info::Config for Runtime {}

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
		cumulus_primitives_core::AggregateMessageOrigin,
	>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	// The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = sp_core::ConstU32<{ 103 * 1024 }>;
	type MaxStale = sp_core::ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = ();
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = ();
	// Enqueue XCMP messages from siblings for later processing.
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = sp_core::ConstU32<1_000>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	type MaxPageSize = ConstU32<{ 1 << 16 }>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
	type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but let's be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = ();
}

#[docify::export(aura_config)]
impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<true>;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const SessionLength: BlockNumber = 6 * HOURS;
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root and the StakingAdmin to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsVoiceOfBody<RelayLocation, StakingAdminBodyId>>,
>;

impl pallet_collator_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type UpdateOrigin = CollatorSelectionUpdateOrigin;
	type PotId = PotId;
	type MaxCandidates = ConstU32<100>;
	type MinEligibleCollators = ConstU32<4>;
	type MaxInvulnerables = ConstU32<20>;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = ();
}

#[derive_impl(pallet_multimap::config_preludes::TestDefaultConfig)]
impl pallet_multimap::Config<Instance1> for Runtime {
	type Key = AccountId;
	type Value = VarAddress;
	type CapacityPerKey = ConstU32<{ VarAddress::variant_count() }>;
}

impl unify_account::Config for Runtime {
	type AddressMap = AddressMap;
	type DrainBalance = Balances;
}

const BLOCK_GAS_LIMIT: u64 = 75_000_000;
const MAX_POV_SIZE: u64 = 5 * 1024 * 1024;
const WEIGHT_MILLISECS_PER_BLOCK: u64 = 2000;

parameter_types! {
	pub BlockGasLimit: U256 = U256::from(BLOCK_GAS_LIMIT);
	pub const ChainId: u64 = 1337;
	pub const GasLimitPovSizeRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(MAX_POV_SIZE);
	pub PrecompilesValue: BabelPrecompiles<Runtime> = BabelPrecompiles::<_>::new();
	pub WeightPerGas: Weight = Weight::from_parts(weight_per_gas(BLOCK_GAS_LIMIT, NORMAL_DISPATCH_RATIO, WEIGHT_MILLISECS_PER_BLOCK), 0);
	pub SuicideQuickClearLimit: u32 = 0;
}

impl frame_babel::ethereum::precompile::Config for Runtime {
	type DispatchValidator = ();
	type DecodeLimit = ConstU32<8>;
	type StorageFilter = ();
}

impl frame_babel::ethereum::Erc20Metadata for Runtime {
	fn name() -> &'static str {
		"Ziggurat"
	}

	fn symbol() -> &'static str {
		"ZIG"
	}

	fn decimals() -> u8 {
		18
	}

	fn is_native_currency() -> bool {
		true
	}
}

impl frame_babel::ethereum::AddressToAssetId<AssetId> for Runtime {
	fn address_to_asset_id(address: H160) -> Option<AssetId> {
		let mut data = [0u8; 4];
		let address_bytes: [u8; 20] = address.into();
		if ASSET_PRECOMPILE_ADDRESS_PREFIX.eq(&address_bytes[0..16]) {
			data.copy_from_slice(&address_bytes[16..20]);
			Some(AssetId::from_be_bytes(data))
		} else {
			None
		}
	}

	fn asset_id_to_address(asset_id: AssetId) -> H160 {
		let mut data = [0u8; 20];
		data[0..16].copy_from_slice(ASSET_PRECOMPILE_ADDRESS_PREFIX);
		data[16..20].copy_from_slice(&asset_id.to_be_bytes());
		H160::from(data)
	}
}

impl pallet_evm::Config for Runtime {
	type FeeCalculator = BaseFee;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAddress<AccountId>;
	type WithdrawOrigin = EnsureAddress<AccountId>;
	type AddressMapping = ethereum::AddressMapping<Self>;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type PrecompilesType = BabelPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type OnChargeTransaction = ();
	type OnCreate = ();
	type FindAuthor = ();
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type SuicideQuickClearLimit = SuicideQuickClearLimit;
	type Timestamp = Timestamp;
	type WeightInfo = pallet_evm::weights::SubstrateWeight<Self>;
}

parameter_types! {
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
	type PostLogContent = PostBlockAndTxnHashes;
	type ExtraDataLength = ConstU32<30>;
}

parameter_types! {
	pub DefaultBaseFeePerGas: U256 = U256::from(1_000_000_000);
	pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		Permill::from_parts(500_000)
	}
	fn upper() -> Permill {
		Permill::from_parts(1_000_000)
	}
}

impl pallet_base_fee::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}

impl pallet_multimap::Config<Instance2> for Runtime {
	type Key = AssetIdOf<Runtime>;
	type Value = DenomOf<Runtime>;
	type CapacityPerKey = ConstU32<1>;
	type KeyHasher = Blake2_128Concat;
	type ValueHasher = Blake2_128Concat;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type AssetIdParameter = codec::Compact<AssetId>;
	type Currency = Balances;
	type CreateOrigin = NeverEnsureOrigin<AccountId>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = ConstU128<500>;
	type AssetAccountDeposit = ConstU128<500>;
	type MetadataDepositBase = ConstU128<0>;
	type MetadataDepositPerByte = ConstU128<0>;
	type ApprovalDeposit = ConstU128<0>;
	type StringLimit = ConstU32<20>;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	type RemoveItemsLimit = ConstU32<1000>;
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

parameter_types! {
	pub SimulationGasLimit: u64 = <Runtime as pallet_cosmos::Config>::WeightToGas::convert(
		RuntimeBlockWeights::get().per_class.get(DispatchClass::Normal).max_total.unwrap_or(RuntimeBlockWeights::get().max_block)
	);
}

impl pallet_cosmos::Config for Runtime {
	type AddressMapping = cosmos::address::AddressMapping<Self>;
	type Balance = Balance;
	type AssetId = AssetId;
	type NativeAsset = Balances;
	type Assets = Assets;
	type NativeDenom = NativeDenom;
	type NativeAssetId = NativeAssetId;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_cosmos::weights::CosmosWeight<Self>;
	type WeightToGas = WeightToGas;
	type MinGasPrices = MinGasPrices;
	type AssetToDenom = AssetToDenom;
	type Context = context::Context;
	type ChainInfo = np_cosmos::traits::CosmosHub;
	type AnteHandler = pallet_cosmos_x_auth::AnteDecorators<Self>;
	type MsgFilter = MsgFilter;
	type MsgServiceRouter = cosmos::msg::MsgServiceRouter<Self>;
	type SigVerifiableTx = SigVerifiableTx;
	type SignModeHandler = SignModeHandler;
	type MaxMemoCharacters = MaxMemoCharacters;
	type TxSigLimit = TxSigLimit;
	type MaxDenomLimit = MaxDenomLimit;
	type SimulationGasLimit = SimulationGasLimit;
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
	pub WasmCostRules: CostRules<Runtime> = Default::default();
}

impl pallet_cosmwasm::Config for Runtime {
	const MAX_FRAMES: u8 = 64;
	type RuntimeEvent = RuntimeEvent;
	type AccountIdExtended = AccountId;
	type PalletId = CosmwasmPalletId;
	type MaxCodeSize = ConstU32<{ 1024 * 1024 }>;
	type MaxInstrumentedCodeSize = ConstU32<{ 2 * 1024 * 1024 }>;
	type MaxMessageSize = ConstU32<{ 64 * 1024 }>;
	type AccountToAddr = cosmos::address::AccountToAddr<Self>;
	type AssetToDenom = AssetToDenom;
	type Balance = Balance;
	type AssetId = AssetId;
	type Assets = Assets;
	type NativeAsset = Balances;
	type ChainInfo = np_cosmos::traits::CosmosHub;
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
	type WeightInfo = pallet_cosmwasm::weights::SubstrateWeight<Self>;

	type PalletHook = Precompiles<Self>;

	type UploadWasmOrigin = EnsureSigned<Self::AccountId>;

	type ExecuteWasmOrigin = EnsureSigned<Self::AccountId>;

	type NativeDenom = NativeDenom;

	type NativeAssetId = NativeAssetId;
}

impl frame_babel::Config for Runtime {
	type AddressMap = AddressMap;
	type AssetMap = AssetMap;
}
