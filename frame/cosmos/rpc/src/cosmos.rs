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

use crate::{internal_error, request_error};
use cosmos_runtime_api::{ChainInfo, CosmosRuntimeApi, SimulateError};
use futures::future::TryFutureExt;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
};
use pallet_cosmos_types::tx::SimulateResponse;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::{sha2_256, Bytes, H256};
use sp_runtime::{traits::Block as BlockT, transaction_validity::TransactionSource};
use std::sync::Arc;

#[rpc(client, server)]
#[async_trait]
pub trait CosmosApi<BlockHash> {
	#[method(name = "cosmos_broadcastTx")]
	async fn broadcast_tx(&self, tx_bytes: Bytes) -> RpcResult<H256>;

	#[method(name = "cosmos_simulate")]
	async fn simulate(&self, tx_bytes: Bytes, at: Option<BlockHash>)
		-> RpcResult<SimulateResponse>;

	#[method(name = "cosmos_chainInfo")]
	async fn chain_info(&self) -> RpcResult<ChainInfo>;
}

pub struct Cosmos<C, P> {
	client: Arc<C>,
	pool: Arc<P>,
}

impl<C, P> Cosmos<C, P> {
	pub fn new(client: Arc<C>, pool: Arc<P>) -> Self {
		Self { client, pool }
	}
}

#[async_trait]
impl<Block, C, P> CosmosApiServer<<Block as BlockT>::Hash> for Cosmos<C, P>
where
	Block: BlockT,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + 'static,
	C::Api: cosmos_runtime_api::CosmosRuntimeApi<Block>,
	P: TransactionPool<Block = Block> + 'static,
{
	async fn broadcast_tx(&self, tx_bytes: Bytes) -> RpcResult<H256> {
		let best_hash = self.client.info().best_hash;
		let extrinsic = self
			.client
			.runtime_api()
			.convert_tx(best_hash, tx_bytes.to_vec())
			.map_err(internal_error)?;

		self.pool
			.submit_one(best_hash, TransactionSource::Local, extrinsic)
			.map_ok(move |_| H256(sha2_256(&tx_bytes)))
			.map_err(internal_error)
			.await
	}

	async fn simulate(
		&self,
		tx_bytes: Bytes,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<SimulateResponse> {
		let at = at.unwrap_or(self.client.info().best_hash);
		self.client
			.runtime_api()
			.simulate(at, tx_bytes.to_vec())
			.map_err(internal_error)?
			.map_err(|e| match e {
				SimulateError::InvalidTransaction =>
					request_error("Simulate transaction failed: Invalid transaction"),
				SimulateError::InternalError(e) => internal_error(format!(
					"Simulate transaction failed: {})",
					String::from_utf8_lossy(&e)
				)),
			})
	}

	async fn chain_info(&self) -> RpcResult<ChainInfo> {
		let best_hash = self.client.info().best_hash;
		self.client.runtime_api().chain_info(best_hash).map_err(internal_error)
	}
}
