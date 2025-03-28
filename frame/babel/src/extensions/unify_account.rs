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

use crate::traits::AccountIdProvider;
use core::marker::PhantomData;
use frame_support::traits::{
	tokens::{fungible, Fortitude, Preservation},
	OnKilledAccount as OnKilledAccountT, OriginTrait,
};
use np_babel::VarAddress;
use np_multimap::traits::UniqueMultimap;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{ecdsa, H256};
use sp_runtime::{
	impl_tx_ext_default,
	traits::{AccountIdConversion, DispatchInfoOf, TransactionExtension, Zero},
	transaction_validity::TransactionSource,
};
use sp_weights::Weight;

type AccountIdOf<T> = <T as AccountIdProvider>::AccountId;

/// A configuration for UnifyAccount signed extension.
pub trait Config: AccountIdProvider<AccountId: From<H256> + TryInto<ecdsa::Public>> {
	/// A map from account to addresses.
	type AddressMap: UniqueMultimap<AccountIdOf<Self>, VarAddress>;
	/// Drain account balance when unifying accounts.
	type DrainBalance: DrainBalance<AccountIdOf<Self>>;
}

/// Unifies the accounts associated with the same public key.
///
/// WARN: This extension should be placed after the `CheckNonce` extension.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnifyAccount<T>(PhantomData<fn() -> T>);

impl<T> Default for UnifyAccount<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T> UnifyAccount<T> {
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

impl<T: Config> UnifyAccount<T> {
	pub fn unify_ecdsa(who: &AccountIdOf<T>) -> Result<(), &'static str> {
		if let Ok(public) = who.clone().try_into() {
			#[cfg(feature = "ethereum")]
			{
				let address = np_babel::EthereumAddress::from(public);
				let interim = address.clone().into_account_truncating();
				T::DrainBalance::drain_balance(&interim, who)?;
				T::AddressMap::try_insert(who, VarAddress::Ethereum(address))
					.map_err(|_| "account unification failed: ethereum")?;
			}
			#[cfg(feature = "cosmos")]
			{
				let address = np_babel::CosmosAddress::from(public);
				let interim = address.clone().into_account_truncating();
				T::DrainBalance::drain_balance(&interim, who)?;
				T::AddressMap::try_insert(who, VarAddress::Cosmos(address))
					.map_err(|_| "account unification failed: cosmos")?;
			}
			#[cfg(feature = "nostr")]
			{
				let address = np_babel::NostrAddress::from(public);
				let interim = address.clone().into_account_truncating();
				T::DrainBalance::drain_balance(&interim, who)?;
				T::AddressMap::try_insert(who, VarAddress::Nostr(address))
					.map_err(|_| "account unification failed: cosmos")?;
			}
		}
		Ok(())
	}
}

impl<T> Clone for UnifyAccount<T> {
	fn clone(&self) -> Self {
		Self(Default::default())
	}
}

impl<T> PartialEq for UnifyAccount<T> {
	fn eq(&self, _: &Self) -> bool {
		true
	}
}

impl<T> Eq for UnifyAccount<T> {}

impl<T> core::fmt::Debug for UnifyAccount<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "UnifyAccount")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut core::fmt::Formatter) -> core::fmt::Result {
		Ok(())
	}
}

impl<T> TransactionExtension<T::RuntimeCall> for UnifyAccount<T>
where
	T: Config + frame_system::Config<AccountId = AccountIdOf<T>>,
{
	const IDENTIFIER: &'static str = "UnifyAccount";
	type Implicit = ();
	type Val = ();
	type Pre = ();

	fn weight(&self, _: &T::RuntimeCall) -> Weight {
		// FIXME
		Weight::from_parts(0, 0)
	}

	fn validate(
		&self,
		origin: <T as frame_system::Config>::RuntimeOrigin,
		_call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Encode,
		_source: TransactionSource,
	) -> sp_runtime::traits::ValidateResult<Self::Val, T::RuntimeCall> {
		if let Some(who) = origin.as_signer() {
			let account = frame_system::Account::<T>::get(who);
			if account.nonce.is_zero() {
				let _ = Self::unify_ecdsa(who);
			}
		}
		Ok((Default::default(), (), origin))
	}

	impl_tx_ext_default!(T::RuntimeCall; prepare);
}

pub trait DrainBalance<AccountId> {
	type Output: Default;

	fn drain_balance(_src: &AccountId, _dest: &AccountId) -> Result<Self::Output, &'static str> {
		Ok(Default::default())
	}
}

impl<AccountId, T> DrainBalance<AccountId> for T
where
	AccountId: Eq,
	T: fungible::Inspect<AccountId> + fungible::Mutate<AccountId>,
{
	type Output = T::Balance;

	fn drain_balance(src: &AccountId, dest: &AccountId) -> Result<Self::Output, &'static str> {
		let amount = T::reducible_balance(src, Preservation::Expendable, Fortitude::Polite);
		T::transfer(src, dest, amount, Preservation::Expendable)
			.map_err(|_| "account draining failed")
			.map(|_| amount)
	}
}

pub struct OnKilledAccount<T: Config>(PhantomData<T>);

impl<T: Config> OnKilledAccountT<T::AccountId> for OnKilledAccount<T> {
	fn on_killed_account(who: &T::AccountId) {
		T::AddressMap::remove_all(who);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use np_runtime::{AccountId32, MultiSigner};

	type AccountId = AccountId32<MultiSigner>;

	struct MockConfig;

	impl Config for MockConfig {
		type AddressMap = np_multimap::in_mem::UniqueMultimap<AccountId, VarAddress>;
		type DrainBalance = ();
	}

	impl AccountIdProvider for MockConfig {
		type AccountId = AccountId;
	}

	fn dev_public() -> ecdsa::Public {
		const_hex::decode_to_array(
			b"02509540919faacf9ab52146c9aa40db68172d83777250b28e4679176e49ccdd9f",
		)
		.unwrap()
		.into()
	}

	#[test]
	fn unify_ecdsa_works() {
		let who = AccountId::from(dev_public());
		let _ = UnifyAccount::<MockConfig>::unify_ecdsa(&who);
		let cosmos = VarAddress::Cosmos(dev_public().into());
		assert_eq!(<MockConfig as Config>::AddressMap::find_key(cosmos), Some(who.clone()));
		let ethereum = VarAddress::Ethereum(dev_public().into());
		assert_eq!(<MockConfig as Config>::AddressMap::find_key(ethereum), Some(who.clone()));
		#[cfg(feature = "nostr")]
		{
			let nostr = VarAddress::Nostr(dev_public().into());
			assert_eq!(<MockConfig as Config>::AddressMap::find_key(nostr), Some(who));
		}
	}

	#[test]
	fn on_killed_account_works() {
		let who = AccountId::from(dev_public());
		let _ = UnifyAccount::<MockConfig>::unify_ecdsa(&who);
		assert!(!<MockConfig as Config>::AddressMap::get(&who).is_empty());
		OnKilledAccount::<MockConfig>::on_killed_account(&who);
		assert!(<MockConfig as Config>::AddressMap::get(&who).is_empty());
	}
}
