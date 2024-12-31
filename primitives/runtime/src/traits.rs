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

pub use sp_runtime::traits::{IdentifyAccount, Lazy};

/// Means of signature verification.
pub trait VerifyMut {
	/// Type of the signer.
	type Signer: IdentifyAccount;

	/// Verify a signature.
	///
	/// Return `true` if signature is valid for the value.
	fn verify_mut<L: Lazy<[u8]>>(
		&self,
		msg: L,
		signer: &mut <Self::Signer as IdentifyAccount>::AccountId,
	) -> bool;
}

/// Validity checker.
pub trait Checkable<T> {
	/// Result of checking.
	type Output;

	/// Checks the validity of a value.
	fn check(&mut self, value: T) -> Self::Output;
}

/// Property accessor.
pub trait Property<T = ()> {
	/// Property value type.
	type Value;

	/// Get the reference to the property.
	fn get(&self) -> &Self::Value;

	/// Set the property.
	fn set(&mut self, value: Self::Value);
}

/// Convert a value from another type, possibly losing information.
pub trait LossyFrom<T>: Sized {
	fn lossy_from(value: T) -> Self;
}

macro_rules! impl_lossy_from {
	($(impl LossyFrom<$from:ty> for $to:ty {})*) => {
		$(
			impl LossyFrom<$from> for $to {
				fn lossy_from(value: $from) -> Self {
					value as Self
				}
			}
		)*
	};
}

impl_lossy_from! {
	impl LossyFrom<u128> for u128 {}

	impl LossyFrom<u128> for u64 {}
	impl LossyFrom<u64> for u64 {}

	impl LossyFrom<u128> for u32 {}
	impl LossyFrom<u64> for u32 {}
	impl LossyFrom<u32> for u32 {}

	impl LossyFrom<u128> for u16 {}
	impl LossyFrom<u64> for u16 {}
	impl LossyFrom<u32> for u16 {}
	impl LossyFrom<u16> for u16 {}

	impl LossyFrom<u128> for u8 {}
	impl LossyFrom<u64> for u8 {}
	impl LossyFrom<u32> for u8 {}
	impl LossyFrom<u16> for u8 {}
	impl LossyFrom<u8> for u8 {}
}

/// Convert a value to another type, possibly losing information.
pub trait LossyInto<T>: Sized {
	fn lossy_into(self) -> T;
}

impl<T, U> LossyInto<U> for T
where
	U: LossyFrom<T>,
{
	fn lossy_into(self) -> U {
		U::lossy_from(self)
	}
}

/// A thin placeholder for late initialization.
///
/// # Panics
///
/// Panics if the value is accessed before initialization.
pub struct LateInit<T>(Option<T>);

impl<T> Default for LateInit<T> {
	fn default() -> Self {
		Self(None)
	}
}

impl<T> LateInit<T> {
	pub const fn new() -> Self {
		Self(None)
	}

	pub fn init(&mut self, value: T) {
		self.0 = Some(value);
	}

	const fn inner(&self) -> &T {
		self.0.as_ref().expect("LateInit not initialized")
	}
}

impl<T: Clone> LateInit<T> {
	pub fn cloned(&self) -> T {
		self.inner().clone()
	}
}

impl<T> AsRef<T> for LateInit<T> {
	fn as_ref(&self) -> &T {
		self.inner()
	}
}

impl<T> core::ops::Deref for LateInit<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.inner()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn property_should_work() {
		struct Foo {
			pub n: i32,
			pub s: String,
		}
		impl Property<i32> for Foo {
			type Value = i32;
			fn get(&self) -> &Self::Value {
				&self.n
			}
			fn set(&mut self, value: Self::Value) {
				self.n = value;
			}
		}
		impl Property<String> for Foo {
			type Value = String;
			fn get(&self) -> &Self::Value {
				&self.s
			}
			fn set(&mut self, value: Self::Value) {
				self.s = value;
			}
		}

		let f = Foo { n: 42, s: "hello".to_string() };

		assert_eq!(*<Foo as Property<i32>>::get(&f), 42);
		assert_eq!(<Foo as Property<String>>::get(&f), "hello");
	}

	#[test]
	fn late_init_should_work() {
		let mut li = <LateInit<i32>>::new();
		let err = std::panic::catch_unwind(|| {
			let _ = li.clone();
		});
		assert!(err.is_err());
		if let Err(e) = err {
			let msg = e.downcast_ref::<String>().unwrap();
			assert_eq!(msg, &"LateInit not initialized");
		}
		li.init(42);
		assert_eq!(*li, 42);
	}
}
