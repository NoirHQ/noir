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

use cosmos_sdk_proto::{
	cosmos::{crypto::secp256k1, tx::v1beta1::Tx},
	prost::Message,
};
use fp_evm::TransactionValidationError;
use fp_self_contained::SelfContainedCall;
use frame_babel::{ethereum::TransactionExt, UnifyAccount};
use np_cosmos::Address as CosmosAddress;
use np_ethereum::Address as EthereumAddress;
use pallet_cosmos_types::{any_match, tx_msgs::FeeTx};
use sp_core::ecdsa;
use sp_runtime::{
	traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf},
	transaction_validity::{
		InvalidTransaction, TransactionValidity, TransactionValidityError, UnknownTransaction,
	},
	DispatchResultWithInfo,
};

use super::{AccountId, Runtime, RuntimeCall, RuntimeOrigin};

impl SelfContainedCall for RuntimeCall {
	type SignedInfo = AccountId;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			RuntimeCall::Cosmos(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => {
				if let pallet_ethereum::Call::transact { transaction } = call {
					let check = || {
						let origin = transaction.recover_key().map(Self::SignedInfo::from).ok_or(
							InvalidTransaction::Custom(
								TransactionValidationError::InvalidSignature as u8,
							),
						)?;
						Ok(origin)
					};
					Some(check())
				} else {
					None
				}
			},
			RuntimeCall::Cosmos(call) =>
				if let pallet_cosmos::Call::transact { tx_bytes } = call {
					let check = || {
						let tx =
							Tx::decode(&mut &tx_bytes[..]).map_err(|_| InvalidTransaction::Call)?;
						let fee_payer = tx.fee_payer().ok_or(InvalidTransaction::Call)?;
						let signer_infos =
							tx.auth_info.ok_or(InvalidTransaction::Call)?.signer_infos;

						let public_key = if fee_payer.is_empty() {
							signer_infos.first()
						} else {
							signer_infos.last()
						}
						.and_then(|signer_info| signer_info.public_key.as_ref())
						.ok_or(InvalidTransaction::Call)?;

						let origin = any_match!(
							public_key, {
								secp256k1::PubKey => {
									let pubkey = secp256k1::PubKey::decode(&mut &*public_key.value).map_err(|_| InvalidTransaction::BadSigner)?;
									ecdsa::Public::try_from(pubkey.key.as_ref()).map(Self::SignedInfo::from).map_err(|_| InvalidTransaction::BadSigner)
								}
							},
							Err(InvalidTransaction::BadSigner)
						)?;

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
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			RuntimeCall::Ethereum(call) => {
				if let pallet_ethereum::Call::transact { transaction } = call {
					if transaction.nonce() == 0 {
						match UnifyAccount::<Runtime>::unify_ecdsa(info) {
							Ok(_) => (),
							Err(_) =>
								return Some(Err(TransactionValidityError::Unknown(
									UnknownTransaction::CannotLookup,
								))),
						}
					}
				}
				let public: ecdsa::Public = info.clone().try_into().unwrap();
				let address: EthereumAddress = public.into();
				call.validate_self_contained(&address.into(), dispatch_info, len)
			},
			RuntimeCall::Cosmos(call) => {
				if let pallet_cosmos::Call::transact { tx_bytes } = call {
					match Self::unify_cosmos_account(tx_bytes) {
						Ok(_) => (),
						Err(e) => return Some(Err(e)),
					};
				};
				let public: ecdsa::Public = info.clone().try_into().unwrap();
				let address: CosmosAddress = public.into();
				call.validate_self_contained(&address.into(), dispatch_info, len)
			},
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => {
				if let pallet_ethereum::Call::transact { transaction } = &call {
					if transaction.nonce() == 0 {
						match UnifyAccount::<Runtime>::unify_ecdsa(info) {
							Ok(_) => (),
							Err(_) =>
								return Some(Err(TransactionValidityError::Unknown(
									UnknownTransaction::CannotLookup,
								))),
						}
					}
				}
				let public: ecdsa::Public = info.clone().try_into().unwrap();
				let address: EthereumAddress = public.into();
				call.pre_dispatch_self_contained(&address.into(), dispatch_info, len)
			},
			RuntimeCall::Cosmos(call) => {
				if let pallet_cosmos::Call::transact { tx_bytes } = call {
					match Self::unify_cosmos_account(tx_bytes) {
						Ok(_) => (),
						Err(e) => return Some(Err(e)),
					};
				};
				call.pre_dispatch_self_contained(dispatch_info, len)
			},
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
				let public: ecdsa::Public = info.clone().try_into().unwrap();
				let address: EthereumAddress = public.into();
				Some(call.dispatch(RuntimeOrigin::from(
					pallet_ethereum::RawOrigin::EthereumTransaction(address.into()),
				)))
			},
			call @ RuntimeCall::Cosmos(pallet_cosmos::Call::transact { .. }) => {
				let public: ecdsa::Public = info.clone().try_into().unwrap();
				let address: CosmosAddress = public.into();
				Some(call.dispatch(RuntimeOrigin::from(
					pallet_cosmos::RawOrigin::CosmosTransaction(address.into()),
				)))
			},
			_ => None,
		}
	}
}

impl RuntimeCall {
	fn unify_cosmos_account(tx_bytes: &[u8]) -> Result<(), TransactionValidityError> {
		let tx = Tx::decode(&mut &*tx_bytes).map_err(|_| InvalidTransaction::Call)?;
		let signer_infos = &tx.auth_info.as_ref().ok_or(InvalidTransaction::Call)?.signer_infos;

		for signer_info in signer_infos.iter() {
			if signer_info.sequence == 0 {
				let public_key = signer_info.public_key.as_ref().ok_or(InvalidTransaction::Call)?;
				let signer = any_match!(
					public_key, {
						secp256k1::PubKey => {
							let pubkey = secp256k1::PubKey::decode(&mut &*public_key.value).map_err(|_| InvalidTransaction::BadSigner)?;
							ecdsa::Public::try_from(pubkey.key.as_ref()).map(AccountId::from).map_err(|_| InvalidTransaction::BadSigner)
						}
					},
					Err(InvalidTransaction::BadSigner)
				)?;

				UnifyAccount::<Runtime>::unify_ecdsa(&signer)
					.map_err(|_| InvalidTransaction::Call)?;
			}
		}

		Ok(())
	}
}
