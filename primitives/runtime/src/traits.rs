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

pub trait LossyFrom<T>: Sized {
	fn lossy_from(value: T) -> Self;
}

macro_rules! impl_lossy_from {
	($($from:ty => $to:ty),*) => {
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
	u128 => u128,
	u128 => u64,
	u128 => u32,
	u128 => u16,
	u128 => u8,
	u64 => u64,
	u64 => u32,
	u64 => u16,
	u64 => u8,
	u32 => u32,
	u32 => u16,
	u32 => u8,
	u16 => u16,
	u16 => u8,
	u8 => u8
}

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
