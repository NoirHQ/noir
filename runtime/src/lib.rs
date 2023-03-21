// This file is part of Noir.

// Copyright (C) 2023 Haderech Pte. Ltd.
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

//! Noir runtime.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod compat;
use compat::ethereum::TransactionExt;
mod precompiles;

use codec::{Decode, Encode};
use fp_ethereum::TransactionValidationError;
use fp_rpc::TransactionStatus;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		tokens::fungible, ConstU128, ConstU32, ConstU64, ConstU8, FindAuthor, KeyOwnerProofSystem,
		OnTimestampSet,
	},
	weights::{
		constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		IdentityFee, Weight,
	},
};
pub use noir_core_primitives::{AccountId, Balance, BlockNumber, Hash, Index, Signature};
use np_crypto::ecdsa::EcdsaExt;
use np_runtime::{AccountName, UniversalAddressKind};
use pallet_ethereum::{Call::transact, PostLogContent, Transaction as EthereumTransaction};
use pallet_evm::{Account as EVMAccount, AddressMapping, FeeCalculator, Runner};
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
use precompiles::FrontierPrecompiles;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
	crypto::{ByteArray, KeyTypeId},
	OpaqueMetadata, H160, H256, U256,
};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		BlakeTwo256, Block as BlockT, DispatchInfoOf, Dispatchable, Get, NumberFor, One,
		PostDispatchInfoOf, UniqueSaturatedInto,
	},
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
		UnknownTransaction,
	},
	ApplyExtrinsicResult, ConsensusEngineId, Perbill, Permill,
};
use sp_std::{marker::PhantomData, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountName>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// Constant values used within the runtime.
pub const MICROUNIT: Balance = 1_000_000;
pub const MILLIUNIT: Balance = 1_000 * MICROUNIT;
pub const UNIT: Balance = 1_000 * MILLIUNIT;
// TODO: Need to set deposit rule.
/// Charge fee for stored bytes and items.
pub const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * UNIT) + (bytes as Balance * 100 * MICROUNIT)
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
	type SignedInfo = AccountId;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) =>
				if let transact { transaction } = call {
					let check = || {
						let origin = transaction
							.recover_key()
							.map(|k| Self::SignedInfo::from(k))
							.ok_or(InvalidTransaction::Custom(
							TransactionValidationError::InvalidSignature as u8,
						))?;

						Ok(origin)
					};

					Some(check())
				} else {
					None
				},
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			RuntimeCall::Ethereum(call) => {
				let address = info.to_eth_address().unwrap();
				if let pallet_ethereum::Call::transact { transaction } = &call {
					if transaction.nonce() == 0 {
						if Runtime::migrate_evm_account(&address, info).is_err() {
							return Some(Err(TransactionValidityError::Unknown(
								UnknownTransaction::CannotLookup,
							)))
						}
					}
				}
				call.validate_self_contained(&info.to_eth_address().unwrap(), dispatch_info, len)
			},
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => {
				let address = info.to_eth_address().unwrap();
				if let pallet_ethereum::Call::transact { transaction } = &call {
					if transaction.nonce() == 0 {
						if Runtime::migrate_evm_account(&address, info).is_err() {
							return Some(Err(TransactionValidityError::Unknown(
								UnknownTransaction::CannotLookup,
							)))
						}
					}
				}
				call.pre_dispatch_self_contained(
					&info.to_eth_address().unwrap(),
					dispatch_info,
					len,
				)
			},
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) =>
				Some(call.dispatch(RuntimeOrigin::from(
					pallet_ethereum::RawOrigin::EthereumTransaction(info.to_eth_address().unwrap()),
				))),
			_ => None,
		}
	}
}

pub mod opaque {
	use super::*;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

/// Runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("noir"),
	impl_name: create_runtime_str!("noir"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// Native version.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX);
pub const MAXIMUM_BLOCK_LENGTH: u32 = 5 * 1024 * 1024;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
		::with_sensible_defaults(MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(MAXIMUM_BLOCK_LENGTH, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

pub struct OnNewAccount;
impl frame_support::traits::OnNewAccount<AccountId> for OnNewAccount {
	fn on_new_account(who: &AccountId) {
		if who.kind() == UniversalAddressKind::Secp256k1 {
			let _ = Runtime::migrate_evm_account(&who.to_eth_address().unwrap(), who);
			let _ =
				pallet_account_alias_registry::Pallet::<Runtime>::connect_aliases_secp256k1(who);
		}
	}
}
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountAliasRegistry;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = OnNewAccount;
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub struct TagGenerator;
impl pallet_account_alias_registry::TagGenerator<Runtime> for TagGenerator {
	fn tag(id: &AccountId, name: &str) -> Result<u16, ()> {
		let salt = pallet_timestamp::Pallet::<Runtime>::get() >> 26u64;
		Self::tag_inner(id.as_ref(), name, salt)
	}
}
impl TagGenerator {
	fn tag_inner(id: &[u8], name: &str, salt: u64) -> Result<u16, ()> {
		let hash = (id, name, salt).using_encoded(sp_io::hashing::blake2_256);

		for index in (0..=(hash.len() - 2)).step_by(2) {
			let num = u16::from_be_bytes(hash[index..index + 2].try_into().unwrap()) % 10000u16;
			if num >= 10 {
				return Ok(num)
			}
		}
		Err(())
	}
}

impl pallet_account_alias_registry::Config for Runtime {
	/// The overarching event type.
	type RuntimeEvent = RuntimeEvent;
	/// Weight information for extrinsics in this pallet.
	type WeightInfo = pallet_account_alias_registry::weights::SubstrateWeight<Runtime>;
	/// The generator for tag number that discriminates the same name accounts.
	type TagGenerator = TagGenerator;
}

impl pallet_aura::Config for Runtime {
	/// The identifier type for an authority.
	type AuthorityId = AuraId;
	/// A way to check whether a given validator is disabled and should not be authoring blocks.
	/// Blocks authored by a disabled validator will lead to a panic as part of this module's
	/// initialization.
	type DisabledValidators = ();
	/// The maximum number of authorities that the pallet can hold.
	type MaxAuthorities = ConstU32<32>;
}

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 500;

impl pallet_balances::Config for Runtime {
	/// The maximum number of locks that should exist on an account.
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	/// The id type for named reserves.
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	/// Handler for the unbalanced reduction when removing a dust account.
	type DustRemoval = ();
	/// The minimum amount required to keep an account open.
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
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
	/// Lower and upper bounds for increasing / decreasing `BaseFeePerGas`.
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}

parameter_types! {
	pub BoundDivision: U256 = U256::from(1024);
}

impl pallet_dynamic_fee::Config for Runtime {
	/// Bound divisor for min gas price.
	type MinGasPriceBoundDivisor = BoundDivision;
}

parameter_types! {
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	/// How Ethereum state root is calculated.
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
	type PostLogContent = PostBlockAndTxnHashes;
}

pub struct FindAuthorTruncated<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		if let Some(author_index) = F::find_author(digests) {
			let authority_id = Aura::authorities()[author_index as usize].clone();
			return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]))
		}
		None
	}
}

const WEIGHT_PER_GAS: u64 = 20_000;
parameter_types! {
	pub BlockGasLimit: U256 = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT.ref_time() / WEIGHT_PER_GAS);
	pub PrecompilesValue: FrontierPrecompiles<Runtime> = FrontierPrecompiles::<_>::new();
	pub WeightPerGas: Weight = Weight::from_ref_time(WEIGHT_PER_GAS);
}

impl pallet_evm::Config for Runtime {
	type FeeCalculator = BaseFee;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
	type CallOrigin = compat::evm::EnsureAddressHashed<AccountId>;
	type WithdrawOrigin = compat::evm::EnsureAddressHashed<AccountId>;
	type AddressMapping = compat::evm::HashedAddressMapping<Self, BlakeTwo256>;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	/// Precompiles associated with this EVM engine.
	type PrecompilesType = FrontierPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = EVMChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type OnChargeTransaction = ();
	type OnCreate = ();
	/// Find author for the current block.
	type FindAuthor = FindAuthorTruncated<Aura>;
}

impl pallet_evm_chain_id::Config for Runtime {}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	/// A system for proving ownership of keys, i.e. that a given key was part
	/// of a validator set, needed for validating equivocation reports.
	type KeyOwnerProofSystem = ();
	/// The proof of key ownership, used for validating equivocation reports
	/// The proof must include the session index and validator count of the
	/// session at which the equivocation occurred.
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	/// The identification of a key owner, used when reporting equivocations.
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;
	/// The equivocation handling subsystem, defines methods to report an
	/// offence (after the equivocation has been validated) and for submitting a
	/// transaction to report an equivocation (from an offchain context).
	type HandleEquivocation = ();
	type WeightInfo = ();
	/// Max Authorities in use.
	type MaxAuthorities = ConstU32<32>;
	type MaxSetIdSessionEntries = ConstU64<0>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
}

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
	pub storage EnableManualSeal: bool = false;
}

pub struct ConsensusOnTimestampSet<T>(PhantomData<T>);
impl<T: pallet_aura::Config> OnTimestampSet<T::Moment> for ConsensusOnTimestampSet<T> {
	fn on_timestamp_set(moment: T::Moment) {
		if EnableManualSeal::get() {
			return
		}
		<pallet_aura::Pallet<T> as OnTimestampSet<T::Moment>>::on_timestamp_set(moment)
	}
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	/// Something which can be notified when the timestamp is set.
	type OnTimestampSet = ConsensusOnTimestampSet<Self>;
	/// The minimum period between blocks.
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	/// Handler for withdrawing, refunding and depositing the transaction fee.
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	/// A fee mulitplier for `Operational` extrinsics to compute "virtual tip" to boost their
	/// `priority`.
	type OperationalFeeMultiplier = ConstU8<5>;
	/// Convert a weight value into a deductible fee based on the currency type.
	type WeightToFee = IdentityFee<Balance>;
	/// Convert a length value into a deductible fee based on the currency type.
	type LengthToFee = IdentityFee<Balance>;
	/// Update the multiplier of the next block, based on the previous block's weight.
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime
	where
		Block = Block,
		NodeBlock = noir_core_primitives::Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		AccountAliasRegistry: pallet_account_alias_registry,
		Aura: pallet_aura,
		Balances: pallet_balances,
		BaseFee: pallet_base_fee,
		DynamicFee: pallet_dynamic_fee,
		Ethereum: pallet_ethereum,
		EVM: pallet_evm,
		EVMChainId: pallet_evm_chain_id,
		Grandpa: pallet_grandpa,
		Sudo: pallet_sudo,
		System: frame_system,
		Timestamp: pallet_timestamp,
		TransactionPayment: pallet_transaction_payment,
	}
);

#[derive(Clone)]
pub struct TransactionConverter;

impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		)
	}
}

impl fp_rpc::ConvertTransaction<sp_runtime::OpaqueExtrinsic> for TransactionConverter {
	fn convert_transaction(
		&self,
		transaction: pallet_ethereum::Transaction,
	) -> sp_runtime::OpaqueExtrinsic {
		let extrinsic = UncheckedExtrinsic::new_unsigned(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		);
		let encoded = extrinsic.encode();
		sp_runtime::OpaqueExtrinsic::decode(&mut &encoded[..])
			.expect("Encoded extrinsic is always valid")
	}
}

impl Runtime {
	fn migrate_evm_account(address: &H160, who: &AccountId) -> Result<Balance, ()> {
		use fungible::{Inspect, Transfer};
		let interim_account =
			<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(*address);
		let balance =
			pallet_balances::Pallet::<Runtime>::reducible_balance(&interim_account, false);
		<pallet_balances::Pallet<Runtime> as Transfer<AccountId>>::transfer(
			&interim_account,
			who,
			balance,
			false,
		)
		.map_err(|_| ())
		.map(|_| balance)
	}
}

impl_runtime_apis! {
	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			<Runtime as pallet_evm::Config>::ChainId::get()
		}

		fn account_basic(address: H160) -> EVMAccount {
			let (account, _) = EVM::account_basic(&address);
			account
		}

		fn gas_price() -> U256 {
			let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
			gas_price
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			EVM::account_codes(address)
		}

		fn author() -> H160 {
			<pallet_evm::Pallet<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
			EVM::account_storages(address, H256::from_slice(&tmp[..]))
		}

		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			let is_transactional = false;
			let validate = true;
			let evm_config = config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config());
			<Runtime as pallet_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				validate,
				evm_config,
			).map_err(|err| err.error.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			let is_transactional = false;
			let validate = true;
			let evm_config = config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config());
			<Runtime as pallet_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				validate,
				evm_config,
			).map_err(|err| err.error.into())
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			Ethereum::current_transaction_statuses()
		}

		fn current_block() -> Option<pallet_ethereum::Block> {
			Ethereum::current_block()
		}

		fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
			Ethereum::current_receipts()
		}

		fn current_all() -> (
			Option<pallet_ethereum::Block>,
			Option<Vec<pallet_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				Ethereum::current_block(),
				Ethereum::current_receipts(),
				Ethereum::current_transaction_statuses()
			)
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
				_ => None
			}).collect::<Vec<EthereumTransaction>>()
		}

		fn elasticity() -> Option<Permill> {
			Some(BaseFee::elasticity())
		}

		fn gas_limit_multiplier_support() {}
	}

	impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
		fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_unsigned(
				pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
			)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}
		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}
		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn tag_inner_test() {
		let now: u64 = 1_676_679_312_000;
		let id = [0u8; 32];
		let name = "test";
		let tag = crate::TagGenerator::tag_inner(&id, name, now).unwrap();
		assert_eq!(tag, 128);
	}
}
