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

use crate::extensions::unify_account;
use core::marker::PhantomData;
use frame_support::dispatch::RawOrigin;
use np_ethereum::Address as EthereumAddress;
use pallet_ethereum::Transaction;
use pallet_evm::{
	EnsureAddressOrigin, IsPrecompileResult, Precompile, PrecompileHandle, PrecompileResult,
	PrecompileSet,
};
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use sp_core::{ecdsa, H160};
use sp_runtime::traits::AccountIdConversion;

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

pub struct FrontierPrecompiles<R>(PhantomData<R>);

impl<R> Default for FrontierPrecompiles<R>
where
	R: pallet_evm::Config,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<R> FrontierPrecompiles<R>
where
	R: pallet_evm::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}
	pub fn used_addresses() -> [H160; 11] {
		[
			hash(1),
			hash(2),
			hash(3),
			hash(4),
			hash(5),
			hash(6),
			hash(7),
			hash(8),
			hash(9),
			hash(1024),
			hash(1025),
		]
	}
}
impl<R> PrecompileSet for FrontierPrecompiles<R>
where
	R: pallet_evm::Config,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
		match handle.code_address() {
			// Ethereum precompiles :
			a if a == hash(1) => Some(ECRecover::execute(handle)),
			a if a == hash(2) => Some(Sha256::execute(handle)),
			a if a == hash(3) => Some(Ripemd160::execute(handle)),
			a if a == hash(4) => Some(Identity::execute(handle)),
			a if a == hash(5) => Some(Modexp::execute(handle)),
			a if a == hash(6) => Some(Bn128Add::execute(handle)),
			a if a == hash(7) => Some(Bn128Mul::execute(handle)),
			a if a == hash(8) => Some(Bn128Pairing::execute(handle)),
			a if a == hash(9) => Some(Blake2F::execute(handle)),
			// Non-Frontier specific nor Ethereum precompiles :
			a if a == hash(1024) => Some(Sha3FIPS256::execute(handle)),
			a if a == hash(1025) => Some(ECRecoverPublicKey::execute(handle)),
			_ => None,
		}
	}

	fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
		IsPrecompileResult::Answer {
			is_precompile: Self::used_addresses().contains(&address),
			extra_cost: 0,
		}
	}
}

fn hash(a: u64) -> H160 {
	H160::from_low_u64_be(a)
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
