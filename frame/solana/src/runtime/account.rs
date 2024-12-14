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

use crate::{runtime::Lamports, Config};
use bincode::{self, EncodeError, Error, ErrorKind};
use nostd::{cell::RefCell, mem::MaybeUninit, ptr, rc::Rc, sync::Arc};
#[cfg(feature = "std")]
use solana_sdk::account::{InheritableAccountFields, DUMMY_INHERITABLE_ACCOUNT_FIELDS};
pub use solana_sdk::account::{ReadableAccount, WritableAccount};
use solana_sdk::{
	account_utils::StateMut, clock::Epoch, instruction::InstructionError, lamports::LamportsError,
	pubkey::Pubkey, sysvar::Sysvar,
};

/// An Account with data that is stored on chain
/// This will be the in-memory representation of the 'Account' struct data.
/// The existing 'Account' structure cannot easily change due to downstream projects.
#[derive(PartialEq, Eq, Clone /* , Deserialize */)]
#[derive_where(Debug)]
//#[serde(from = "Account")]
pub struct AccountSharedData<T: Config> {
	/// lamports in the account
	pub(super) lamports: Lamports<T>,
	/// data held in this account
	pub(super) data: Arc<Vec<u8>>,
	/// the program that owns this account. If executable, the program that loads this account.
	pub(super) owner: Pubkey,
	/// this account's data contains a loaded program (and is now read-only)
	pub(super) executable: bool,
	/// the epoch at which this account will next owe rent
	pub(super) rent_epoch: Epoch,
}

impl<T: Config> Default for AccountSharedData<T> {
	fn default() -> Self {
		Self {
			lamports: Default::default(),
			data: Default::default(),
			owner: Default::default(),
			executable: Default::default(),
			rent_epoch: Default::default(),
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> From<solana_sdk::account::Account> for AccountSharedData<T> {
	fn from(other: solana_sdk::account::Account) -> Self {
		Self {
			lamports: other.lamports.into(),
			data: Arc::new(other.data),
			owner: other.owner,
			executable: other.executable,
			rent_epoch: other.rent_epoch,
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> From<AccountSharedData<T>> for solana_sdk::account::Account {
	fn from(mut other: AccountSharedData<T>) -> Self {
		let account_data = Arc::make_mut(&mut other.data);
		Self {
			lamports: other.lamports.get(),
			data: std::mem::take(account_data),
			owner: other.owner,
			executable: other.executable,
			rent_epoch: other.rent_epoch,
		}
	}
}

impl<T: Config> From<solana_sdk::account::AccountSharedData> for AccountSharedData<T> {
	fn from(other: solana_sdk::account::AccountSharedData) -> Self {
		solana_sdk::account::Account::from(other).into()
	}
}

impl<T: Config> AccountSharedData<T> {
	pub fn get_lamports(&self) -> Lamports<T> {
		self.lamports.clone()
	}

	pub fn set_lamports(&mut self, lamports: Lamports<T>) {
		self.lamports = lamports;
	}

	pub fn is_shared(&self) -> bool {
		Arc::strong_count(&self.data) > 1
	}

	pub fn reserve(&mut self, additional: usize) {
		if let Some(data) = Arc::get_mut(&mut self.data) {
			data.reserve(additional)
		} else {
			let mut data = Vec::with_capacity(self.data.len().saturating_add(additional));
			data.extend_from_slice(&self.data);
			self.data = Arc::new(data);
		}
	}

	pub fn capacity(&self) -> usize {
		self.data.capacity()
	}

	fn data_mut(&mut self) -> &mut Vec<u8> {
		Arc::make_mut(&mut self.data)
	}

	pub fn resize(&mut self, new_len: usize, value: u8) {
		self.data_mut().resize(new_len, value)
	}

	pub fn extend_from_slice(&mut self, data: &[u8]) {
		self.data_mut().extend_from_slice(data)
	}

	pub fn set_data_from_slice(&mut self, new_data: &[u8]) {
		// If the buffer isn't shared, we're going to memcpy in place.
		let Some(data) = Arc::get_mut(&mut self.data) else {
			// If the buffer is shared, the cheapest thing to do is to clone the
			// incoming slice and replace the buffer.
			return self.set_data(new_data.to_vec());
		};

		let new_len = new_data.len();

		// Reserve additional capacity if needed. Here we make the assumption
		// that growing the current buffer is cheaper than doing a whole new
		// allocation to make `new_data` owned.
		//
		// This assumption holds true during CPI, especially when the account
		// size doesn't change but the account is only changed in place. And
		// it's also true when the account is grown by a small margin (the
		// realloc limit is quite low), in which case the allocator can just
		// update the allocation metadata without moving.
		//
		// Shrinking and copying in place is always faster than making
		// `new_data` owned, since shrinking boils down to updating the Vec's
		// length.

		data.reserve(new_len.saturating_sub(data.len()));

		// Safety:
		// We just reserved enough capacity. We set data::len to 0 to avoid
		// possible UB on panic (dropping uninitialized elements), do the copy,
		// finally set the new length once everything is initialized.
		#[allow(clippy::uninit_vec)]
		// this is a false positive, the lint doesn't currently special case set_len(0)
		unsafe {
			data.set_len(0);
			ptr::copy_nonoverlapping(new_data.as_ptr(), data.as_mut_ptr(), new_len);
			data.set_len(new_len);
		};
	}

	//#[cfg_attr(feature = "dev-context-only-utils", qualifiers(pub))]
	pub(crate) fn set_data(&mut self, data: Vec<u8>) {
		self.data = Arc::new(data);
	}

	pub fn spare_data_capacity_mut(&mut self) -> &mut [MaybeUninit<u8>] {
		self.data_mut().spare_capacity_mut()
	}

	pub fn new(lamports: u64, space: usize, owner: &Pubkey) -> Self {
		Self::new_strict(lamports.into(), space, owner)
	}
	pub fn new_strict(lamports: Lamports<T>, space: usize, owner: &Pubkey) -> Self {
		Self { lamports, data: Arc::new(vec![0; space]), owner: *owner, ..Default::default() }
	}
	pub fn new_ref(lamports: u64, space: usize, owner: &Pubkey) -> Rc<RefCell<Self>> {
		Self::new_ref_strict(lamports.into(), space, owner)
	}
	pub fn new_ref_strict(
		lamports: Lamports<T>,
		space: usize,
		owner: &Pubkey,
	) -> Rc<RefCell<Self>> {
		Rc::new(RefCell::new(Self::new_strict(lamports, space, owner)))
	}
	pub fn new_data<U: serde::Serialize>(
		lamports: u64,
		state: &U,
		owner: &Pubkey,
	) -> Result<Self, bincode::Error> {
		let data = bincode::serialize(state)?;
		Ok(Self {
			lamports: lamports.into(),
			data: Arc::new(data),
			owner: *owner,
			..Default::default()
		})
	}
	pub fn new_data_strict<U: serde::Serialize>(
		lamports: Lamports<T>,
		state: &U,
		owner: &Pubkey,
	) -> Result<Self, bincode::Error> {
		let data = bincode::serialize(state)?;
		Ok(Self { lamports, data: Arc::new(data), owner: *owner, ..Default::default() })
	}
	pub fn new_ref_data<U: serde::Serialize>(
		lamports: u64,
		state: &U,
		owner: &Pubkey,
	) -> Result<RefCell<Self>, bincode::Error> {
		Self::new_ref_data_strict(lamports.into(), state, owner)
	}
	pub fn new_ref_data_strict<U: serde::Serialize>(
		lamports: Lamports<T>,
		state: &U,
		owner: &Pubkey,
	) -> Result<RefCell<Self>, bincode::Error> {
		Ok(RefCell::new(Self::new_data_strict(lamports, state, owner)?))
	}
	pub fn new_data_with_space<U: serde::Serialize>(
		lamports: u64,
		state: &U,
		space: usize,
		owner: &Pubkey,
	) -> Result<Self, bincode::Error> {
		Self::new_data_with_space_strict(lamports.into(), state, space, owner)
	}
	pub fn new_data_with_space_strict<U: serde::Serialize>(
		lamports: Lamports<T>,
		state: &U,
		space: usize,
		owner: &Pubkey,
	) -> Result<Self, bincode::Error> {
		let mut account = Self::new_strict(lamports, space, owner);

		account.serialize_data(state)?;

		Ok(account)
	}
	pub fn new_ref_data_with_space<U: serde::Serialize>(
		lamports: u64,
		state: &U,
		space: usize,
		owner: &Pubkey,
	) -> Result<RefCell<Self>, bincode::Error> {
		Self::new_ref_data_with_space_strict(lamports.into(), state, space, owner)
	}
	pub fn new_ref_data_with_space_strict<U: serde::Serialize>(
		lamports: Lamports<T>,
		state: &U,
		space: usize,
		owner: &Pubkey,
	) -> Result<RefCell<Self>, bincode::Error> {
		Ok(RefCell::new(Self::new_data_with_space_strict(lamports, state, space, owner)?))
	}
	pub fn new_rent_epoch(lamports: u64, space: usize, owner: &Pubkey, rent_epoch: Epoch) -> Self {
		Self::new_rent_epoch_strict(lamports.into(), space, owner, rent_epoch)
	}
	pub fn new_rent_epoch_strict(
		lamports: Lamports<T>,
		space: usize,
		owner: &Pubkey,
		rent_epoch: Epoch,
	) -> Self {
		Self {
			lamports,
			data: Arc::new(vec![0; space]),
			owner: *owner,
			rent_epoch,
			..Default::default()
		}
	}
	pub fn deserialize_data<U: serde::de::DeserializeOwned>(&self) -> Result<U, bincode::Error> {
		bincode::deserialize(self.data())
	}
	pub fn serialize_data<U: serde::Serialize>(&mut self, state: &U) -> Result<(), bincode::Error> {
		if bincode::serialized_size(state)? > self.data().len() as u64 {
			return Err(EncodeError::UnexpectedEnd.into());
		}
		bincode::serialize_into(self.data_as_mut_slice(), state)
	}

	// NOTE: [`AccountSharedData`] doesn't implement WritableAccount intentionally
	// not to lose precision when inner balance has different decimals with lamports.
	//
	// impl<T: Config> WritableAccount for AccountSharedData<T> {
	#[cfg(feature = "std")]
	pub fn checked_add_lamports(&mut self, lamports: u64) -> Result<(), LamportsError> {
		self.set_lamports(
			self.get_lamports()
				.checked_add(lamports)
				.ok_or(LamportsError::ArithmeticOverflow)?,
		);
		Ok(())
	}
	#[cfg(feature = "std")]
	pub fn checked_sub_lamports(&mut self, lamports: u64) -> Result<(), LamportsError> {
		self.set_lamports(
			self.get_lamports()
				.checked_sub(lamports)
				.ok_or(LamportsError::ArithmeticUnderflow)?,
		);
		Ok(())
	}
	pub fn data_as_mut_slice(&mut self) -> &mut [u8] {
		&mut self.data_mut()[..]
	}
	pub fn set_owner(&mut self, owner: Pubkey) {
		self.owner = owner;
	}
	pub fn copy_into_owner_from_slice(&mut self, source: &[u8]) {
		self.owner.as_mut().copy_from_slice(source);
	}
	pub fn set_executable(&mut self, executable: bool) {
		self.executable = executable;
	}
	pub fn set_rent_epoch(&mut self, epoch: Epoch) {
		self.rent_epoch = epoch;
	}
	// }
}

impl<T: Config> ReadableAccount for AccountSharedData<T> {
	fn lamports(&self) -> u64 {
		self.lamports.get()
	}
	fn data(&self) -> &[u8] {
		&self.data
	}
	fn owner(&self) -> &Pubkey {
		&self.owner
	}
	fn executable(&self) -> bool {
		self.executable
	}
	fn rent_epoch(&self) -> Epoch {
		self.rent_epoch
	}
}

impl<T: Config, U> StateMut<U> for AccountSharedData<T>
where
	U: serde::Serialize + serde::de::DeserializeOwned,
{
	fn state(&self) -> Result<U, InstructionError> {
		self.deserialize_data().map_err(|_| InstructionError::InvalidAccountData)
	}
	fn set_state(&mut self, state: &U) -> Result<(), InstructionError> {
		self.serialize_data(state).map_err(|err| match *err {
			ErrorKind::Encode(EncodeError::UnexpectedEnd) => InstructionError::AccountDataTooSmall,
			_ => InstructionError::GenericError,
		})
	}
}

#[cfg(feature = "std")]
pub fn create_account_shared_data_for_test<S: Sysvar, T: Config>(
	sysvar: &S,
) -> AccountSharedData<T> {
	solana_sdk::account::create_account_shared_data_for_test(sysvar).into()
}

#[cfg(test)]
pub mod tests {
	use crate::mock::AccountSharedData;
	use solana_sdk::{
		account::{accounts_equal, Account, ReadableAccount, WritableAccount},
		account_utils::StateMut,
		instruction::InstructionError,
		pubkey::Pubkey,
	};

	fn make_two_accounts(key: &Pubkey) -> (Account, AccountSharedData) {
		let mut account1 = Account::new(1, 2, key);
		account1.executable = true;
		account1.rent_epoch = 4;
		let mut account2 = AccountSharedData::new(1, 2, key);
		account2.executable = true;
		account2.rent_epoch = 4;
		assert!(accounts_equal(&account1, &account2));
		(account1, account2)
	}

	#[test]
	fn test_account_data_copy_as_slice() {
		let key = Pubkey::new_unique();
		let key2 = Pubkey::new_unique();
		let (mut account1, mut account2) = make_two_accounts(&key);
		account1.copy_into_owner_from_slice(key2.as_ref());
		account2.copy_into_owner_from_slice(key2.as_ref());
		assert!(accounts_equal(&account1, &account2));
		assert_eq!(account1.owner(), &key2);
	}

	#[test]
	fn test_account_set_data_from_slice() {
		let key = Pubkey::new_unique();
		let (_, mut account) = make_two_accounts(&key);
		assert_eq!(account.data(), &vec![0, 0]);
		account.set_data_from_slice(&[1, 2]);
		assert_eq!(account.data(), &vec![1, 2]);
		account.set_data_from_slice(&[1, 2, 3]);
		assert_eq!(account.data(), &vec![1, 2, 3]);
		account.set_data_from_slice(&[4, 5, 6]);
		assert_eq!(account.data(), &vec![4, 5, 6]);
		account.set_data_from_slice(&[4, 5, 6, 0]);
		assert_eq!(account.data(), &vec![4, 5, 6, 0]);
		account.set_data_from_slice(&[]);
		assert_eq!(account.data().len(), 0);
		account.set_data_from_slice(&[44]);
		assert_eq!(account.data(), &vec![44]);
		account.set_data_from_slice(&[44]);
		assert_eq!(account.data(), &vec![44]);
	}

	#[test]
	fn test_account_data_set_data() {
		let key = Pubkey::new_unique();
		let (_, mut account) = make_two_accounts(&key);
		assert_eq!(account.data(), &vec![0, 0]);
		account.set_data(vec![1, 2]);
		assert_eq!(account.data(), &vec![1, 2]);
		account.set_data(vec![]);
		assert_eq!(account.data().len(), 0);
	}

	#[test]
	#[should_panic(
		expected = "called `Result::unwrap()` on an `Err` value: Io(Kind(UnexpectedEof))"
	)]
	fn test_account_deserialize() {
		let key = Pubkey::new_unique();
		let (account1, _account2) = make_two_accounts(&key);
		account1.deserialize_data::<String>().unwrap();
	}

	#[test]
	#[should_panic(expected = "called `Result::unwrap()` on an `Err` value: SizeLimit")]
	fn test_account_serialize() {
		let key = Pubkey::new_unique();
		let (mut account1, _account2) = make_two_accounts(&key);
		account1.serialize_data(&"hello world").unwrap();
	}

	#[test]
	#[should_panic(
		expected = "called `Result::unwrap()` on an `Err` value: Io(Kind(UnexpectedEof))"
	)]
	fn test_account_shared_data_deserialize() {
		let key = Pubkey::new_unique();
		let (_account1, account2) = make_two_accounts(&key);
		account2.deserialize_data::<String>().unwrap();
	}

	#[test]
	#[should_panic(expected = "called `Result::unwrap()` on an `Err` value: SizeLimit")]
	fn test_account_shared_data_serialize() {
		let key = Pubkey::new_unique();
		let (_account1, mut account2) = make_two_accounts(&key);
		account2.serialize_data(&"hello world").unwrap();
	}

	#[test]
	fn test_to_account_shared_data() {
		let key = Pubkey::new_unique();
		let (account1, account2) = make_two_accounts(&key);
		assert!(accounts_equal(&account1, &account2));
		let account3 = account1.to_account_shared_data();
		let account4 = account2.to_account_shared_data();
		assert!(accounts_equal(&account1, &account3));
		assert!(accounts_equal(&account1, &account4));
	}

	#[test]
	fn test_account_shared_data() {
		let key = Pubkey::new_unique();
		let (account1, account2) = make_two_accounts(&key);
		assert!(accounts_equal(&account1, &account2));
		let account = account1;
		assert_eq!(account.lamports, 1);
		assert_eq!(account.lamports(), 1);
		assert_eq!(account.data.len(), 2);
		assert_eq!(account.data().len(), 2);
		assert_eq!(account.owner, key);
		assert_eq!(account.owner(), &key);
		assert!(account.executable);
		assert!(account.executable());
		assert_eq!(account.rent_epoch, 4);
		assert_eq!(account.rent_epoch(), 4);
		let account = account2;
		assert_eq!(account.lamports, 1.into());
		assert_eq!(account.lamports(), 1);
		assert_eq!(account.data.len(), 2);
		assert_eq!(account.data().len(), 2);
		assert_eq!(account.owner, key);
		assert_eq!(account.owner(), &key);
		assert!(account.executable);
		assert!(account.executable());
		assert_eq!(account.rent_epoch, 4);
		assert_eq!(account.rent_epoch(), 4);
	}

	// test clone and from for both types against expected
	fn test_equal(
		should_be_equal: bool,
		account1: &Account,
		account2: &AccountSharedData,
		account_expected: &Account,
	) {
		assert_eq!(should_be_equal, accounts_equal(account1, account2));
		if should_be_equal {
			assert!(accounts_equal(account_expected, account2));
		}
		assert_eq!(
			accounts_equal(account_expected, account1),
			accounts_equal(account_expected, &account1.clone())
		);
		assert_eq!(
			accounts_equal(account_expected, account2),
			accounts_equal(account_expected, &account2.clone())
		);
		assert_eq!(
			accounts_equal(account_expected, account1),
			accounts_equal(account_expected, &AccountSharedData::from(account1.clone()))
		);
		assert_eq!(
			accounts_equal(account_expected, account2),
			accounts_equal(account_expected, &Account::from(account2.clone()))
		);
	}

	#[test]
	fn test_account_add_sub_lamports() {
		let key = Pubkey::new_unique();
		let (mut account1, mut account2) = make_two_accounts(&key);
		assert!(accounts_equal(&account1, &account2));
		account1.checked_add_lamports(1).unwrap();
		account2.checked_add_lamports(1).unwrap();
		assert!(accounts_equal(&account1, &account2));
		assert_eq!(account1.lamports(), 2);
		account1.checked_sub_lamports(2).unwrap();
		account2.checked_sub_lamports(2).unwrap();
		assert!(accounts_equal(&account1, &account2));
		assert_eq!(account1.lamports(), 0);
	}

	#[test]
	#[should_panic(expected = "Overflow")]
	fn test_account_checked_add_lamports_overflow() {
		let key = Pubkey::new_unique();
		let (mut account1, _account2) = make_two_accounts(&key);
		account1.checked_add_lamports(u64::MAX).unwrap();
	}

	#[test]
	#[should_panic(expected = "Underflow")]
	fn test_account_checked_sub_lamports_underflow() {
		let key = Pubkey::new_unique();
		let (mut account1, _account2) = make_two_accounts(&key);
		account1.checked_sub_lamports(u64::MAX).unwrap();
	}

	#[test]
	#[should_panic(expected = "Overflow")]
	fn test_account_checked_add_lamports_overflow2() {
		let key = Pubkey::new_unique();
		let (_account1, mut account2) = make_two_accounts(&key);
		account2.checked_add_lamports(u64::MAX).unwrap();
	}

	#[test]
	#[should_panic(expected = "Underflow")]
	fn test_account_checked_sub_lamports_underflow2() {
		let key = Pubkey::new_unique();
		let (_account1, mut account2) = make_two_accounts(&key);
		account2.checked_sub_lamports(u64::MAX).unwrap();
	}

	#[test]
	fn test_account_saturating_add_lamports() {
		let key = Pubkey::new_unique();
		let (mut account, _) = make_two_accounts(&key);

		let remaining = 22;
		account.set_lamports(u64::MAX - remaining);
		account.saturating_add_lamports(remaining * 2);
		assert_eq!(account.lamports(), u64::MAX);
	}

	#[test]
	fn test_account_saturating_sub_lamports() {
		let key = Pubkey::new_unique();
		let (mut account, _) = make_two_accounts(&key);

		let remaining = 33;
		account.set_lamports(remaining);
		account.saturating_sub_lamports(remaining * 2);
		assert_eq!(account.lamports(), 0);
	}

	#[test]
	fn test_account_shared_data_all_fields() {
		let key = Pubkey::new_unique();
		let key2 = Pubkey::new_unique();
		let key3 = Pubkey::new_unique();
		let (mut account1, mut account2) = make_two_accounts(&key);
		assert!(accounts_equal(&account1, &account2));

		let mut account_expected = account1.clone();
		assert!(accounts_equal(&account1, &account_expected));
		assert!(accounts_equal(&account1, &account2.clone())); // test the clone here

		for field_index in 0..5 {
			for pass in 0..4 {
				if field_index == 0 {
					if pass == 0 {
						account1.checked_add_lamports(1).unwrap();
					} else if pass == 1 {
						account_expected.checked_add_lamports(1).unwrap();
						account2.set_lamports(account2.lamports + 1);
					} else if pass == 2 {
						account1.set_lamports(account1.lamports + 1);
					} else if pass == 3 {
						account_expected.checked_add_lamports(1).unwrap();
						account2.checked_add_lamports(1).unwrap();
					}
				} else if field_index == 1 {
					if pass == 0 {
						account1.data[0] += 1;
					} else if pass == 1 {
						account_expected.data[0] += 1;
						account2.data_as_mut_slice()[0] = account2.data[0] + 1;
					} else if pass == 2 {
						account1.data_as_mut_slice()[0] = account1.data[0] + 1;
					} else if pass == 3 {
						account_expected.data[0] += 1;
						account2.data_as_mut_slice()[0] += 1;
					}
				} else if field_index == 2 {
					if pass == 0 {
						account1.owner = key2;
					} else if pass == 1 {
						account_expected.owner = key2;
						account2.set_owner(key2);
					} else if pass == 2 {
						account1.set_owner(key3);
					} else if pass == 3 {
						account_expected.owner = key3;
						account2.owner = key3;
					}
				} else if field_index == 3 {
					if pass == 0 {
						account1.executable = !account1.executable;
					} else if pass == 1 {
						account_expected.executable = !account_expected.executable;
						account2.set_executable(!account2.executable);
					} else if pass == 2 {
						account1.set_executable(!account1.executable);
					} else if pass == 3 {
						account_expected.executable = !account_expected.executable;
						account2.executable = !account2.executable;
					}
				} else if field_index == 4 {
					if pass == 0 {
						account1.rent_epoch += 1;
					} else if pass == 1 {
						account_expected.rent_epoch += 1;
						account2.set_rent_epoch(account2.rent_epoch + 1);
					} else if pass == 2 {
						account1.set_rent_epoch(account1.rent_epoch + 1);
					} else if pass == 3 {
						account_expected.rent_epoch += 1;
						account2.rent_epoch += 1;
					}
				}

				let should_be_equal = pass == 1 || pass == 3;
				test_equal(should_be_equal, &account1, &account2, &account_expected);

				// test new_ref
				if should_be_equal {
					assert!(accounts_equal(
						&Account::new_ref(
							account_expected.lamports(),
							account_expected.data().len(),
							account_expected.owner()
						)
						.borrow(),
						&*AccountSharedData::new_ref(
							account_expected.lamports(),
							account_expected.data().len(),
							account_expected.owner()
						)
						.borrow()
					));

					{
						// test new_data
						let account1_with_data = Account::new_data(
							account_expected.lamports(),
							&account_expected.data()[0],
							account_expected.owner(),
						)
						.unwrap();
						let account2_with_data = AccountSharedData::new_data(
							account_expected.lamports(),
							&account_expected.data()[0],
							account_expected.owner(),
						)
						.unwrap();

						assert!(accounts_equal(&account1_with_data, &account2_with_data));
						assert_eq!(
							account1_with_data.deserialize_data::<u8>().unwrap(),
							account2_with_data.deserialize_data::<u8>().unwrap()
						);
					}

					// test new_data_with_space
					assert!(accounts_equal(
						&Account::new_data_with_space(
							account_expected.lamports(),
							&account_expected.data()[0],
							1,
							account_expected.owner()
						)
						.unwrap(),
						&AccountSharedData::new_data_with_space(
							account_expected.lamports(),
							&account_expected.data()[0],
							1,
							account_expected.owner()
						)
						.unwrap()
					));

					// test new_ref_data
					assert!(accounts_equal(
						&Account::new_ref_data(
							account_expected.lamports(),
							&account_expected.data()[0],
							account_expected.owner()
						)
						.unwrap()
						.borrow(),
						&*AccountSharedData::new_ref_data(
							account_expected.lamports(),
							&account_expected.data()[0],
							account_expected.owner()
						)
						.unwrap()
						.borrow()
					));

					//new_ref_data_with_space
					assert!(accounts_equal(
						&Account::new_ref_data_with_space(
							account_expected.lamports(),
							&account_expected.data()[0],
							1,
							account_expected.owner()
						)
						.unwrap()
						.borrow(),
						&*AccountSharedData::new_ref_data_with_space(
							account_expected.lamports(),
							&account_expected.data()[0],
							1,
							account_expected.owner()
						)
						.unwrap()
						.borrow()
					));
				}
			}
		}
	}

	#[test]
	fn test_account_state() {
		let state = 42u64;

		assert!(AccountSharedData::default().set_state(&state).is_err());
		let res = AccountSharedData::default().state() as Result<u64, InstructionError>;
		assert!(res.is_err());

		let mut account = AccountSharedData::new(0, std::mem::size_of::<u64>(), &Pubkey::default());

		assert!(account.set_state(&state).is_ok());
		let stored_state: u64 = account.state().unwrap();
		assert_eq!(stored_state, state);
	}
}
