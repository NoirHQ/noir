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

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::RuntimeDebug;

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

pub trait AddressMapping<A> {
	fn into_account_id(address: H160) -> A;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use cosmos_sdk_proto::Any;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungibles,
			tokens::{AssetId, Balance},
			Contains, Currency,
		},
	};
	use frame_system::WeightInfo;
	use np_cosmos::traits::ChainInfo;
	use pallet_cosmos_types::{
		context::traits::Context, events::CosmosEvent, gas::Gas, handler::AnteDecorator,
		msgservice::MsgServiceRouter,
	};
	use pallet_cosmos_x_auth_signing::{
		sign_mode_handler::traits::SignModeHandler, sign_verifiable_tx::traits::SigVerifiableTx,
	};
	use sp_runtime::traits::Convert;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event {
		AnteHandled(Vec<CosmosEvent>),
		Executed { gas_wanted: u64, gas_used: u64, events: Vec<CosmosEvent> },
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
}
