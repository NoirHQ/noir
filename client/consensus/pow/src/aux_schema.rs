// This file is part of Noir.

// Copyright (C) Haderech Pte. Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use np_consensus_pow::BlockWeight;
use parity_scale_codec::{Decode, Encode};
use sc_client_api::AuxStore;
use sp_blockchain::{Error, Result};

pub fn block_weight_key<H: Encode>(block_hash: H) -> Vec<u8> {
	(b"block_weight", block_hash).encode()
}

fn load_decode<B: AuxStore, T: Decode>(backend: &B, key: &[u8]) -> Result<Option<T>> {
	match backend.get_aux(key)? {
		None => Ok(None),
		Some(t) => T::decode(&mut &t[..])
			.map_err(|e| Error::Backend(format!("PoW DB is corrupted: {}", e)))
			.map(Some),
	}
}

pub fn load_block_weight<B, H, W>(backend: &B, block_hash: &H) -> Result<BlockWeight<W>>
where
	B: AuxStore,
	H: Encode,
	W: Decode + Default,
{
	load_decode(backend, &block_weight_key(block_hash)[..]).map(|v| v.unwrap_or_default())
}

pub(crate) fn write_block_weight<H, W, F, R>(
	block_hash: H,
	block_weight: BlockWeight<W>,
	write_aux: F,
) -> R
where
	H: Encode,
	W: Encode,
	F: FnOnce(&[(Vec<u8>, &[u8])]) -> R,
{
	let key = block_weight_key(block_hash);
	block_weight.using_encoded(|s| write_aux(&[(key, s)]))
}
