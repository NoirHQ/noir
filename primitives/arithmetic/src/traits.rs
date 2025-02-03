// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
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

pub use sp_arithmetic::traits::*;

use core::ops::{Div, Mul};
use sp_core::{U256, U512};

/// Trait for multiplication and division with saturating.
pub trait SaturatingMulDiv<Rhs = Self> {
	/// Calculates `(self * num) / denom` with saturating.
	fn saturating_mul_div(self, num: Rhs, denom: Rhs) -> Self;
}

type Promoted<T, Rhs> = <T as Promotion<Rhs>>::Output;
type WidenPromoted<T, Rhs> = <Promoted<T, Rhs> as Widen>::Output;

impl<T, Rhs> SaturatingMulDiv<Rhs> for T
where
	T: Promotion<Rhs> + UpperBounded,
	Promoted<T, Rhs>: Widen,
	WidenPromoted<T, Rhs>:
		Mul<Output = WidenPromoted<T, Rhs>> + Div<Output = WidenPromoted<T, Rhs>>,
{
	fn saturating_mul_div(self, num: Rhs, denom: Rhs) -> Self {
		let this = <Promoted<T, Rhs>>::from(self).widen();
		let num = <Promoted<T, Rhs>>::from(num).widen();
		let denom = <Promoted<T, Rhs>>::from(denom).widen();

		match (this * num / denom).try_into() {
			Ok(v) => v.try_into().unwrap_or(T::max_value()),
			Err(_) => T::max_value(),
		}
	}
}

/// Workaround for the lack of impl Bounded for U256.
trait UpperBounded {
	fn max_value() -> Self;
}

macro_rules! impl_upper_bounded {
	($t:ty) => {
		impl UpperBounded for $t {
			fn max_value() -> Self {
				<$t>::MAX
			}
		}
	};
}

impl_upper_bounded!(u8);
impl_upper_bounded!(u16);
impl_upper_bounded!(u32);
impl_upper_bounded!(u64);
impl_upper_bounded!(u128);
impl_upper_bounded!(U256);

/// Trait for promoting type to larger one between two types.
trait Promotion<Rhs>: Sized {
	type Output: From<Self> + From<Rhs> + TryInto<Self>;
}

impl<T> Promotion<T> for T {
	type Output = T;
}

macro_rules! impl_promotion {
	($t:ty, $u:ty, $v: ty) => {
		impl Promotion<$u> for $t {
			type Output = $v;
		}
	};
	($t:ty, {$($u:ty),*}, {$($v:ty),*}) => {
		$(impl_promotion!($t, $u, $t);)*
		$(impl_promotion!($t, $v, $v);)*
	};
}

impl_promotion!(u8, {}, {u16, u32, u64, u128, U256});
impl_promotion!(u16, {u8}, {u32, u64, u128, U256});
impl_promotion!(u32, {u8, u16}, {u64, u128, U256});
impl_promotion!(u64, {u8, u16, u32}, {u128, U256});
impl_promotion!(u128, {u8, u16, u32, u64}, {U256});
impl_promotion!(U256, {u8, u16, u32, u64, u128}, {});

/// Trait for widening type.
trait Widen: Sized {
	type Output: From<Self> + TryInto<Self>;

	/// Widen a value to a larger type.
	fn widen(self) -> Self::Output {
		Self::Output::from(self)
	}
}
impl Widen for u8 {
	type Output = u16;
}
impl Widen for u16 {
	type Output = u32;
}
impl Widen for u32 {
	type Output = u64;
}
impl Widen for u64 {
	type Output = u128;
}
impl Widen for u128 {
	type Output = U256;
}
impl Widen for U256 {
	type Output = U512;
}

#[cfg(test)]
mod tests {
	use super::*;

	struct CustomNumeric(pub u8);

	impl From<CustomNumeric> for U256 {
		fn from(value: CustomNumeric) -> Self {
			U256::from(value.0)
		}
	}
	impl TryFrom<U256> for CustomNumeric {
		type Error = &'static str;
		fn try_from(value: U256) -> Result<Self, Self::Error> {
			value.try_into().map(CustomNumeric)
		}
	}
	impl UpperBounded for CustomNumeric {
		fn max_value() -> Self {
			CustomNumeric(u8::max_value())
		}
	}
	impl core::ops::Mul for CustomNumeric {
		type Output = Self;
		fn mul(self, rhs: Self) -> Self {
			CustomNumeric(self.0.wrapping_mul(rhs.0))
		}
	}
	impl core::ops::Div for CustomNumeric {
		type Output = Self;
		fn div(self, rhs: Self) -> Self {
			CustomNumeric(self.0.wrapping_div(rhs.0))
		}
	}
	impl Promotion<U256> for CustomNumeric {
		type Output = U256;
	}
	impl Promotion<CustomNumeric> for U256 {
		type Output = U256;
	}

	#[test]
	fn saturating_mul_div_with_custom_type() {
		// custom lhs
		assert_eq!(127, CustomNumeric(254).saturating_mul_div(U256::from(2), U256::from(4)).0);

		// custom rhs
		let m = CustomNumeric(254);
		let d = CustomNumeric(4);
		assert_eq!(U256::from(127), U256::from(2).saturating_mul_div(m, d));
	}
}
