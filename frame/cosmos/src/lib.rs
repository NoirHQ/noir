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

extern crate alloc;

pub mod weights;

pub use self::pallet::*;
use crate::weights::WeightInfo;
use cosmos_sdk_proto::{cosmos::tx::v1beta1::Tx, traits::Message};
use frame_support::{
	dispatch::{DispatchErrorWithPostInfo, DispatchInfo, PostDispatchInfo},
	pallet_prelude::*,
};
use frame_system::{pallet_prelude::*, CheckWeight};
use pallet_cosmos_types::{
	address::acc_address_from_bech32, context::traits::Context, errors::RootError,
	events::traits::EventManager, gas::traits::GasMeter, handler::AnteDecorator,
	msgservice::MsgServiceRouter,
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
				let tx = Tx::decode(&mut &tx_bytes[..])
					.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;
				let fee_payer = T::SigVerifiableTx::fee_payer(&tx)
					.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;
				let (_hrp, address_raw) = acc_address_from_bech32(&fee_payer).map_err(|_| {
					TransactionValidityError::Invalid(InvalidTransaction::BadSigner)
				})?;

				if address_raw.len() != 20 {
					return Err(TransactionValidityError::Invalid(InvalidTransaction::BadSigner));
				}

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
	use frame_support::traits::{
		fungibles,
		tokens::{AssetId, Balance},
		Contains, Currency,
	};
	use np_cosmos::traits::ChainInfo;
	use pallet_cosmos_types::{errors::CosmosError, events::CosmosEvent, gas::Gas};
	use pallet_cosmos_x_auth_signing::sign_mode_handler::traits::SignModeHandler;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event {
		AnteHandled(Vec<CosmosEvent>),
		Executed { gas_wanted: u64, gas_used: u64, events: Vec<CosmosEvent> },
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

		#[pallet::no_default_bounds]
		type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for extrinsics in this pallet.
		#[pallet::no_default]
		type WeightInfo: WeightInfo;

		/// Converter between Weight and Gas.
		type WeightToGas: Convert<Weight, Gas> + Convert<Gas, Weight>;

		/// Converter between AssetId and Denom.
		#[pallet::no_default]
		type AssetToDenom: Convert<String, Result<Self::AssetId, ()>>
			+ Convert<Self::AssetId, String>;

		/// Context for executing transactions.
		type Context: Context;

		type ChainInfo: ChainInfo;

		/// Ante handler for fee and auth.
		type AnteHandler: AnteDecorator;

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
				.map_or(T::WeightInfo::default_weight(), |fee| {
					T::WeightToGas::convert(fee.gas_limit)
				})
		 })]
		pub fn transact(origin: OriginFor<T>, tx_bytes: Vec<u8>) -> DispatchResultWithPostInfo {
			let _source = ensure_cosmos_transaction(origin)?;

			let tx = Tx::decode(&mut &*tx_bytes).map_err(|_| DispatchErrorWithPostInfo {
				post_info: PostDispatchInfo {
					actual_weight: Some(T::WeightInfo::default_weight()),
					pays_fee: Pays::Yes,
				},
				error: Error::<T>::CosmosError(RootError::TxDecodeError.into()).into(),
			})?;

			Self::apply_validated_transaction(tx)
		}
	}
}

impl<T: Config> Pallet<T> {
	fn validate_transaction_in_pool(origin: H160, tx_bytes: &[u8]) -> TransactionValidity {
		let tx = Tx::decode(&mut &*tx_bytes)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;

		T::AnteHandler::ante_handle(&tx, true)?;

		let transaction_nonce = T::SigVerifiableTx::sequence(&tx)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;

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
		let tx = Tx::decode(&mut &*tx_bytes)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;

		T::AnteHandler::ante_handle(&tx, false)?;

		Ok(())
	}

	pub fn apply_validated_transaction(tx: Tx) -> DispatchResultWithPostInfo {
		let body = tx.body.ok_or(DispatchErrorWithPostInfo {
			post_info: PostDispatchInfo {
				actual_weight: Some(T::WeightInfo::default_weight()),
				pays_fee: Pays::Yes,
			},
			error: Error::<T>::CosmosError(RootError::TxDecodeError.into()).into(),
		})?;
		let gas_limit = tx
			.auth_info
			.as_ref()
			.and_then(|auth_info| auth_info.fee.as_ref())
			.ok_or(DispatchErrorWithPostInfo {
				post_info: PostDispatchInfo {
					actual_weight: Some(T::WeightInfo::default_weight()),
					pays_fee: Pays::Yes,
				},
				error: Error::<T>::CosmosError(RootError::TxDecodeError.into()).into(),
			})?
			.gas_limit;

		let mut ctx = T::Context::new(gas_limit);
		ctx.gas_meter()
			.consume_gas(T::WeightInfo::default_weight().ref_time(), "")
			.map_err(|_| DispatchErrorWithPostInfo {
				post_info: PostDispatchInfo {
					actual_weight: Some(Weight::from_parts(ctx.gas_meter().consumed_gas(), 0)),
					pays_fee: Pays::Yes,
				},
				error: Error::<T>::CosmosError(RootError::OutOfGas.into()).into(),
			})?;

		for msg in body.messages.iter() {
			let handler = T::MsgServiceRouter::route(msg).ok_or(DispatchErrorWithPostInfo {
				post_info: PostDispatchInfo {
					actual_weight: Some(Weight::from_parts(ctx.gas_meter().consumed_gas(), 0)),
					pays_fee: Pays::Yes,
				},
				error: Error::<T>::CosmosError(RootError::UnknownRequest.into()).into(),
			})?;

			handler.handle(msg, &mut ctx).map_err(|e| DispatchErrorWithPostInfo {
				post_info: PostDispatchInfo {
					actual_weight: Some(Weight::from_parts(ctx.gas_meter().consumed_gas(), 0)),
					pays_fee: Pays::Yes,
				},
				error: Error::<T>::CosmosError(e).into(),
			})?;
		}

		Self::deposit_event(Event::Executed {
			gas_wanted: gas_limit,
			gas_used: T::WeightToGas::convert(Weight::from_parts(
				ctx.gas_meter().consumed_gas(),
				0,
			)),
			events: ctx.event_manager().events(),
		});

		Ok(PostDispatchInfo {
			actual_weight: Some(Weight::from_parts(ctx.gas_meter().consumed_gas(), 0)),
			pays_fee: Pays::Yes,
		})
	}
}
