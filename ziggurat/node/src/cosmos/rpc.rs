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

use ziggurat_runtime::opaque::Block;

use jsonrpsee::RpcModule;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use std::{error::Error, sync::Arc};

pub struct FullDeps<C, P> {
	pub client: Arc<C>,
	pub pool: Arc<P>,
}

pub fn create_full<C, P>(
	deps: FullDeps<C, P>,
) -> Result<RpcModule<()>, Box<dyn Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool<Block = Block> + 'static,
	C::Api: cosmos_runtime_api::CosmosRuntimeApi<Block>,
	C::Api: cosmwasm_runtime_api::CosmwasmRuntimeApi<Block, Vec<u8>>,
{
	use cosmos_rpc::cosmos::{Cosmos, CosmosApiServer};
	use cosmwasm_rpc::{Cosmwasm, CosmwasmApiServer};

	let mut module = RpcModule::new(());
	let FullDeps { client, pool } = deps;

	module.merge(Cosmos::new(client.clone(), pool).into_rpc())?;
	module.merge(Cosmwasm::new(client).into_rpc())?;

	Ok(module)
}
