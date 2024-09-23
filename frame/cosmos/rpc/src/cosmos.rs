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

use crate::{internal_error, request_error};
use cosmos_runtime_api::{CosmosRuntimeApi, SimulateError, SimulateResponse};
use futures::future::TryFutureExt;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
};
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
				SimulateError::InvalidTx => request_error("Invalid tx"),
				SimulateError::InternalError(e) => internal_error(String::from_utf8_lossy(&e)),
			})
	}
}
