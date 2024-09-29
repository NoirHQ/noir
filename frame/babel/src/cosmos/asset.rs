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

use alloc::string::{String, ToString};
use core::marker::PhantomData;
use pallet_cosmos::types::{AssetIdOf, DenomOf};
use pallet_multimap::traits::UniqueMap;
use sp_core::Get;
use sp_runtime::traits::TryConvert;

pub struct AssetToDenom<T, I>(PhantomData<(T, I)>);
impl<T, I: 'static> TryConvert<String, AssetIdOf<T>> for AssetToDenom<T, I>
where
	T: pallet_cosmos::Config + pallet_multimap::Config<I, Key = AssetIdOf<T>, Value = DenomOf<T>>,
{
	fn try_convert(denom: String) -> Result<AssetIdOf<T>, String> {
		if denom == T::NativeDenom::get() {
			Ok(T::NativeAssetId::get())
		} else {
			let denom_raw: DenomOf<T> =
				denom.as_bytes().to_vec().try_into().map_err(|_| denom.clone())?;
			pallet_multimap::Pallet::<T, I>::find_key(denom_raw).ok_or(denom.clone())
		}
	}
}
impl<T, I: 'static> TryConvert<AssetIdOf<T>, String> for AssetToDenom<T, I>
where
	T: pallet_cosmos::Config + pallet_multimap::Config<I, Key = AssetIdOf<T>, Value = DenomOf<T>>,
{
	fn try_convert(asset_id: AssetIdOf<T>) -> Result<String, AssetIdOf<T>> {
		if asset_id == T::NativeAssetId::get() {
			Ok(T::NativeDenom::get().to_string())
		} else {
			let denom =
				<pallet_multimap::Pallet<T, I> as UniqueMap<AssetIdOf<T>, DenomOf<T>>>::get(
					asset_id.clone(),
				)
				.ok_or(asset_id.clone())?;
			String::from_utf8(denom.into()).map_err(|_| asset_id.clone())
		}
	}
}
