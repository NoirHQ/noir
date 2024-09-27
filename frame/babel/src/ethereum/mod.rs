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

pub mod precompile;
mod precompiles;

pub use precompiles::*;

use crate::*;

use crate::extensions::unify_account;
use core::marker::PhantomData;
use frame_support::dispatch::RawOrigin;
use np_babel::EthereumAddress;
use pallet_ethereum::Transaction;
use pallet_evm::EnsureAddressOrigin;
use pallet_multimap::traits::UniqueMultimap;
use sp_core::{ecdsa, H160};
use sp_runtime::traits::AccountIdConversion;

pub use pallet_evm_precompile_balances_erc20::Erc20Metadata;
pub use pallet_evm_precompileset_assets_erc20::AddressToAssetId;

pub struct EnsureAddress<AccountId>(PhantomData<AccountId>);

impl<OuterOrigin, AccountId> EnsureAddressOrigin<OuterOrigin> for EnsureAddress<AccountId>
where
	OuterOrigin: Into<Result<RawOrigin<AccountId>, OuterOrigin>> + From<RawOrigin<AccountId>>,
	AccountId: TryInto<ecdsa::Public> + Clone,
{
	type Success = AccountId;

	fn try_address_origin(
		address: &H160,
		origin: OuterOrigin,
	) -> Result<Self::Success, OuterOrigin> {
		origin.into().and_then(|o| match o {
			RawOrigin::Signed(who) => {
				if let Ok(key) = who.clone().try_into() {
					if EthereumAddress::from(*address) == key.into() {
						return Ok(who)
					}
				}
				Err(OuterOrigin::from(RawOrigin::Signed(who)))
			},
			r => Err(OuterOrigin::from(r)),
		})
	}
}

pub struct AddressMapping<T>(PhantomData<T>);

impl<T> pallet_evm::AddressMapping<T::AccountId> for AddressMapping<T>
where
	T: unify_account::Config,
{
	fn into_account_id(who: H160) -> T::AccountId {
		let address = EthereumAddress::from(who);
		T::AddressMap::find_key(Address::Ethereum(address.clone()))
			.unwrap_or_else(|| address.into_account_truncating())
	}
}

pub trait TransactionExt {
	fn recover_key(&self) -> Option<ecdsa::Public>;
	fn nonce(&self) -> u64;
}

impl TransactionExt for Transaction {
	fn recover_key(&self) -> Option<ecdsa::Public> {
		let mut sig = [0u8; 65];
		let mut msg = [0u8; 32];
		match self {
			Transaction::Legacy(t) => {
				sig[0..32].copy_from_slice(&t.signature.r()[..]);
				sig[32..64].copy_from_slice(&t.signature.s()[..]);
				sig[64] = t.signature.standard_v();
				msg.copy_from_slice(
					&::ethereum::LegacyTransactionMessage::from(t.clone()).hash()[..],
				);
			},
			Transaction::EIP2930(t) => {
				sig[0..32].copy_from_slice(&t.r[..]);
				sig[32..64].copy_from_slice(&t.s[..]);
				sig[64] = t.odd_y_parity as u8;
				msg.copy_from_slice(
					&::ethereum::EIP2930TransactionMessage::from(t.clone()).hash()[..],
				);
			},
			Transaction::EIP1559(t) => {
				sig[0..32].copy_from_slice(&t.r[..]);
				sig[32..64].copy_from_slice(&t.s[..]);
				sig[64] = t.odd_y_parity as u8;
				msg.copy_from_slice(
					&::ethereum::EIP1559TransactionMessage::from(t.clone()).hash()[..],
				);
			},
		}
		sp_io::crypto::secp256k1_ecdsa_recover_compressed(&sig, &msg)
			.map(ecdsa::Public::from)
			.ok()
	}

	fn nonce(&self) -> u64 {
		match self {
			Transaction::Legacy(t) => t.nonce.as_u64(),
			Transaction::EIP2930(t) => t.nonce.as_u64(),
			Transaction::EIP1559(t) => t.nonce.as_u64(),
		}
	}
}
