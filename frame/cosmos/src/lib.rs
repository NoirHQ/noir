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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unreachable_patterns)]

pub mod types;
pub mod weights;

pub use self::pallet::*;
use crate::weights::WeightInfo;
use cosmos_sdk_proto::{cosmos::tx::v1beta1::Tx, traits::Message};
use frame_support::{
	dispatch::{DispatchInfo, PostDispatchInfo, WithPostDispatchInfo},
	pallet_prelude::*,
};
use frame_system::{pallet_prelude::*, CheckWeight};
use pallet_cosmos_types::{
	address::acc_address_from_bech32,
	context::traits::Context,
	errors::{CosmosError, RootError},
	events::traits::EventManager,
	gas::traits::GasMeter,
	handler::{AnteDecorator, PostDecorator},
	msgservice::traits::MsgServiceRouter,
	tx::{GasInfo, SimulateResponse},
	tx_msgs::FeeTx,
};
use pallet_cosmos_x_auth_signing::sign_verifiable_tx::traits::SigVerifiableTx;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::{
	traits::{Convert, DispatchInfoOf, Dispatchable},
	transaction_validity::ValidTransactionBuilder,
	RuntimeDebug, SaturatedConversion,
};

#[derive(Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum RawOrigin {
	CosmosTransaction(H160),
}

pub fn ensure_cosmos_transaction<OuterOrigin>(o: OuterOrigin) -> Result<H160, &'static str>
where
	OuterOrigin: Into<Result<RawOrigin, OuterOrigin>>,
{
	match o.into() {
		Ok(RawOrigin::CosmosTransaction(n)) => Ok(n),
		_ => Err("bad origin: expected to be a Cosmos transaction"),
	}
}

impl<T> Call<T>
where
	OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	T: Send + Sync + Config,
	T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
{
	pub fn is_self_contained(&self) -> bool {
		matches!(self, Call::transact { .. })
	}

	pub fn check_self_contained(&self) -> Option<Result<H160, TransactionValidityError>> {
		if let Call::transact { tx_bytes } = self {
			let check = || {
				let tx = Tx::decode(&mut &tx_bytes[..]).map_err(|_| InvalidTransaction::Call)?;

				let fee_payer =
					T::SigVerifiableTx::fee_payer(&tx).map_err(|_| InvalidTransaction::Call)?;
				let (_hrp, address_raw) = acc_address_from_bech32(&fee_payer)
					.map_err(|_| InvalidTransaction::BadSigner)?;
				ensure!(address_raw.len() == 20, InvalidTransaction::BadSigner);

				Ok(H160::from_slice(&address_raw))
			};

			Some(check())
		} else {
			None
		}
	}

	pub fn validate_self_contained(
		&self,
		origin: &H160,
		dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		len: usize,
	) -> Option<TransactionValidity> {
		if let Call::transact { tx_bytes } = self {
			if let Err(e) = CheckWeight::<T>::do_validate(dispatch_info, len) {
				return Some(Err(e));
			}

			Some(Pallet::<T>::validate_transaction_in_pool(*origin, tx_bytes))
		} else {
			None
		}
	}

	pub fn pre_dispatch_self_contained(
		&self,
		dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		if let Call::transact { tx_bytes } = self {
			if let Err(e) = CheckWeight::<T>::do_pre_dispatch(dispatch_info, len) {
				return Some(Err(e));
			}

			Some(Pallet::<T>::validate_transaction_in_block(tx_bytes))
		} else {
			None
		}
	}
}

pub trait AddressMapping<A> {
	fn into_account_id(address: H160) -> A;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use cosmos_sdk_proto::Any;
	use frame_support::{
		dispatch::WithPostDispatchInfo,
		traits::{
			fungibles,
			tokens::{AssetId, Balance},
			Contains, Currency,
		},
	};
	use nostd::{string::String, vec::Vec};
	use np_cosmos::traits::ChainInfo;
	use pallet_cosmos_types::{
		context::traits::MinGasPrices, errors::CosmosError, events::CosmosEvent, gas::Gas,
		handler::PostDecorator,
	};
	use pallet_cosmos_x_auth_signing::sign_mode_handler::traits::SignModeHandler;
	use sp_runtime::traits::{Convert, TryConvert};

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::origin]
	pub type Origin = RawOrigin;

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event {
		AnteHandled(Vec<CosmosEvent>),
		Executed { gas_wanted: u64, gas_used: u64, events: Vec<CosmosEvent> },
		PostHandled(Vec<CosmosEvent>),
	}

	#[pallet::error]
	pub enum Error<T> {
		CosmosError(CosmosError),
	}

	#[pallet::config(with_default)]
	pub trait Config: frame_system::Config {
		/// Mapping from address to account id.
		#[pallet::no_default]
		type AddressMapping: AddressMapping<Self::AccountId>;

		type Balance: Balance + Into<u128>;

		/// Identifier for the class of asset.
		type AssetId: AssetId + Ord + MaybeSerializeDeserialize;

		#[pallet::no_default]
		type NativeAsset: Currency<Self::AccountId>;

		#[pallet::no_default]
		type Assets: fungibles::metadata::Inspect<
				Self::AccountId,
				Balance = Self::Balance,
				AssetId = Self::AssetId,
			> + fungibles::Mutate<Self::AccountId, Balance = Self::Balance, AssetId = Self::AssetId>
			+ fungibles::Balanced<Self::AccountId, Balance = Self::Balance, AssetId = Self::AssetId>;

		#[pallet::no_default]
		type NativeDenom: Get<&'static str>;

		type NativeAssetId: Get<Self::AssetId>;

		#[pallet::no_default_bounds]
		type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Converter between Weight and Gas.
		type WeightToGas: Convert<Weight, Gas> + Convert<Gas, Weight>;

		#[pallet::no_default]
		type MinGasPrices: MinGasPrices;

		/// Converter between AssetId and Denom.
		#[pallet::no_default]
		type AssetToDenom: TryConvert<Self::AssetId, String> + TryConvert<String, Self::AssetId>;

		/// Context for executing transactions.
		type Context: Context;

		#[pallet::no_default]
		type ChainInfo: ChainInfo;

		/// Ante handler for fee and auth.
		type AnteHandler: AnteDecorator;

		/// Post handler to perform custom post-processing.
		type PostHandler: PostDecorator<Self::Context>;

		/// Message filter for allowed messages.
		type MsgFilter: Contains<Any>;

		/// Router for redirecting messages.
		#[pallet::no_default]
		type MsgServiceRouter: MsgServiceRouter<Self::Context>;

		#[pallet::no_default]
		type SigVerifiableTx: SigVerifiableTx;

		/// Generate sign bytes according to the sign mode.
		#[pallet::no_default]
		type SignModeHandler: SignModeHandler;

		/// The maximum number of characters allowed in a memo.
		#[pallet::constant]
		type MaxMemoCharacters: Get<u64>;

		/// The maximum number of signatures for a transaction.
		#[pallet::constant]
		type TxSigLimit: Get<u64>;

		/// The maximum length of a denomination for an asset.
		#[pallet::constant]
		type MaxDenomLimit: Get<u32>;

		/// The gas limit for simulation.
		#[pallet::constant]
		type SimulationGasLimit: Get<u64>;
	}

	pub mod config_preludes {
		use super::*;
		use frame_support::{derive_impl, parameter_types, traits::Everything};
		use frame_system::limits::BlockWeights;
		use pallet_cosmos_types::context::Context;

		pub struct WeightToGas;
		impl Convert<Weight, Gas> for WeightToGas {
			fn convert(weight: Weight) -> Gas {
				weight.ref_time()
			}
		}
		impl Convert<Gas, Weight> for WeightToGas {
			fn convert(gas: Gas) -> Weight {
				Weight::from_parts(gas, 0)
			}
		}

		parameter_types! {
			pub const MaxMemoCharacters: u64 = 256;
			pub const TxSigLimit: u64 = 7;
			pub const MaxDenomLimit: u32 = 128;
			pub const NativeAssetId: u32 = 0;
			pub SimulationGasLimit: u64 = BlockWeights::default().base_block.ref_time();
		}

		pub struct TestDefaultConfig;
		#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig, no_aggregated_types)]
		impl frame_system::DefaultConfig for TestDefaultConfig {}

		#[register_default_impl(TestDefaultConfig)]
		impl DefaultConfig for TestDefaultConfig {
			#[inject_runtime_type]
			type RuntimeEvent = ();
			type Balance = u128;
			type AssetId = u32;
			type NativeAssetId = NativeAssetId;
			type WeightToGas = WeightToGas;
			type Context = Context;
			type AnteHandler = ();
			type PostHandler = ();
			type MaxMemoCharacters = MaxMemoCharacters;
			type TxSigLimit = TxSigLimit;
			type MaxDenomLimit = MaxDenomLimit;
			type MsgFilter = Everything;
			type WeightInfo = ();
			type SimulationGasLimit = SimulationGasLimit;
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight({
			use cosmos_sdk_proto::traits::Message;
			use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx;

			Tx::decode(&mut &tx_bytes[..])
				.ok()
				.and_then(|tx| tx.auth_info)
				.and_then(|auth_info| auth_info.fee)
				.map_or(T::WeightInfo::base_weight(), |fee| {
					T::WeightToGas::convert(fee.gas_limit)
				})
		 })]
		pub fn transact(origin: OriginFor<T>, tx_bytes: Vec<u8>) -> DispatchResultWithPostInfo {
			let _source = ensure_cosmos_transaction(origin)?;

			let tx = Tx::decode(&mut &*tx_bytes).map_err(|_| {
				Error::<T>::CosmosError(RootError::TxDecodeError.into())
					.with_weight(T::WeightInfo::base_weight())
			})?;

			Self::apply_validated_transaction(tx)
		}
	}
}

impl<T: Config> Pallet<T> {
	fn validate_transaction_in_pool(origin: H160, tx_bytes: &[u8]) -> TransactionValidity {
		let tx = Tx::decode(&mut &*tx_bytes).map_err(|_| InvalidTransaction::Call)?;

		T::AnteHandler::ante_handle(&tx, true)?;

		let transaction_nonce =
			T::SigVerifiableTx::sequence(&tx).map_err(|_| InvalidTransaction::Call)?;

		let mut builder =
			ValidTransactionBuilder::default().and_provides((origin, transaction_nonce));

		let who = T::AddressMapping::into_account_id(origin);
		let sequence = frame_system::Pallet::<T>::account_nonce(&who).saturated_into();

		if transaction_nonce > sequence {
			if let Some(prev_nonce) = transaction_nonce.checked_sub(1) {
				builder = builder.and_requires((origin, prev_nonce))
			}
		}

		builder.build()
	}

	pub fn validate_transaction_in_block(tx_bytes: &[u8]) -> Result<(), TransactionValidityError> {
		let tx = Tx::decode(&mut &*tx_bytes).map_err(|_| InvalidTransaction::Call)?;

		T::AnteHandler::ante_handle(&tx, false)?;

		Ok(())
	}

	pub fn apply_validated_transaction(tx: Tx) -> DispatchResultWithPostInfo {
		let gas_limit = tx.gas().ok_or(
			Error::<T>::CosmosError(RootError::TxDecodeError.into())
				.with_weight(T::WeightInfo::base_weight()),
		)?;

		let mut ctx = T::Context::new(gas_limit);
		Self::run_tx(&mut ctx, &tx).map_err(|e| {
			Error::<T>::CosmosError(e)
				.with_weight(T::WeightToGas::convert(ctx.gas_meter().consumed_gas()))
		})?;

		T::PostHandler::post_handle(&mut ctx, &tx, false).map_err(|e| {
			Error::<T>::CosmosError(e)
				.with_weight(T::WeightToGas::convert(ctx.gas_meter().consumed_gas()))
		})?;

		Self::deposit_event(Event::Executed {
			gas_wanted: gas_limit,
			gas_used: ctx.gas_meter().consumed_gas(),
			events: ctx.event_manager().events(),
		});

		Ok(PostDispatchInfo {
			actual_weight: Some(T::WeightToGas::convert(ctx.gas_meter().consumed_gas())),
			pays_fee: Pays::Yes,
		})
	}

	pub fn run_tx(ctx: &mut T::Context, tx: &Tx) -> Result<(), CosmosError> {
		let base_gas = T::WeightToGas::convert(T::WeightInfo::base_weight());
		ctx.gas_meter()
			.consume_gas(base_gas, "base_gas")
			.map_err(|_| RootError::OutOfGas)?;

		let body = tx.body.as_ref().ok_or(RootError::TxDecodeError)?;

		for msg in body.messages.iter() {
			let handler = T::MsgServiceRouter::route(msg).ok_or(RootError::UnknownRequest)?;
			handler.handle(ctx, msg)?;
		}

		Ok(())
	}

	pub fn simulate(tx_bytes: Vec<u8>) -> Result<SimulateResponse, CosmosError> {
		let tx = Tx::decode(&mut &*tx_bytes).map_err(|_| RootError::TxDecodeError)?;

		T::AnteHandler::ante_handle(&tx, true)?;

		let mut ctx = T::Context::new(T::SimulationGasLimit::get());
		Self::run_tx(&mut ctx, &tx)?;

		T::PostHandler::post_handle(&mut ctx, &tx, true)?;

		Ok(SimulateResponse {
			gas_info: GasInfo { gas_wanted: 0, gas_used: ctx.gas_meter().consumed_gas() },
			events: ctx.event_manager().events(),
		})
	}
}
