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

mod backend;
mod config;
pub mod rpc;

pub use backend::*;
pub use config::*;

use fc_mapping_sync::{kv::MappingSyncWorker, SyncStrategy};
use fc_rpc::{EthTask, StorageOverride};
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fp_rpc::EthereumRuntimeRPCApi;
use futures::prelude::*;
use jsonrpsee::RpcModule;
use sc_client_api::{
	backend::{Backend, StateBackend, StorageProvider},
	blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata},
	client::BlockchainEvents,
	BlockOf,
};
use sc_network_sync::SyncingService;
use sc_rpc_api::DenyUnsafe;
use sc_service::{Error as ServiceError, TaskManager};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT, Header as HeaderT};
use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
	time::Duration,
};

pub struct SpawnTasksParams<'a, B: BlockT, C, BE> {
	pub config: sc_service::Configuration,
	pub rpc_builder: Box<dyn Fn(DenyUnsafe) -> Result<RpcModule<()>, ServiceError>>,
	pub task_manager: &'a mut TaskManager,
	pub client: Arc<C>,
	pub substrate_backend: Arc<BE>,
	pub frontier_backend: Arc<fc_db::Backend<B, C>>,
	pub filter_pool: Option<FilterPool>,
	pub storage_override: Arc<dyn StorageOverride<B>>,
	pub fee_history_cache: FeeHistoryCache,
	pub fee_history_cache_limit: FeeHistoryCacheLimit,
	pub sync: Arc<SyncingService<B>>,
	pub pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<B>,
		>,
	>,
}

pub fn spawn_tasks<B, C, BE>(
	params: SpawnTasksParams<B, C, BE>,
) -> Result<sc_service::Configuration, sc_service::Error>
where
	C: ProvideRuntimeApi<B> + BlockOf,
	C: HeaderBackend<B> + HeaderMetadata<B, Error = BlockChainError> + 'static,
	C: BlockchainEvents<B> + StorageProvider<B, BE>,
	C: Send + Sync + 'static,
	C::Api: EthereumRuntimeRPCApi<B>,
	C::Api: BlockBuilder<B>,
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	B::Header: HeaderT<Number = u32>,
	BE: Backend<B> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
{
	let SpawnTasksParams {
		mut config,
		rpc_builder,
		task_manager,
		client,
		substrate_backend,
		frontier_backend,
		filter_pool,
		storage_override,
		fee_history_cache,
		fee_history_cache_limit,
		sync,
		pubsub_notification_sinks,
	} = params;

	let rpc_port = config.rpc_addr.map(|addr| addr.port()).unwrap_or(config.rpc_port);
	let prometheus_config = config.prometheus_config.take();

	// TODO: Make the Ethereum RPC port configurable.
	let _ = config.rpc_addr.as_mut().map(|addr| addr.set_port(8545));

	let rpc = sc_service::start_rpc_servers(
		&config,
		rpc_builder,
		Some(Box::new(fc_rpc::EthereumSubIdProvider)),
	);
	if rpc.is_ok() {
		log::info!(
			"Ethereum RPC started: {}",
			config.rpc_addr.as_ref().map(|addr| addr.port()).unwrap_or(0)
		);
	} else {
		log::warn!("Ethereum RPC not started");
	}
	task_manager.keep_alive(rpc);

	// Spawn main mapping sync worker background task.
	match *frontier_backend {
		fc_db::Backend::KeyValue(ref b) => {
			task_manager.spawn_essential_handle().spawn(
				"frontier-mapping-sync-worker",
				Some("frontier"),
				MappingSyncWorker::new(
					client.import_notification_stream(),
					Duration::new(6, 0),
					client.clone(),
					substrate_backend.clone(),
					storage_override.clone(),
					b.clone(),
					3,
					0,
					SyncStrategy::Parachain,
					sync.clone(),
					pubsub_notification_sinks.clone(),
				)
				.for_each(|()| future::ready(())),
			);
		},
		fc_db::Backend::Sql(ref b) => {
			task_manager.spawn_essential_handle().spawn_blocking(
				"frontier-mapping-sync-worker",
				Some("frontier"),
				fc_mapping_sync::sql::SyncWorker::run(
					client.clone(),
					substrate_backend.clone(),
					b.clone(),
					client.import_notification_stream(),
					fc_mapping_sync::sql::SyncWorkerConfig {
						read_notification_timeout: Duration::from_secs(10),
						check_indexed_blocks_interval: Duration::from_secs(60),
					},
					fc_mapping_sync::SyncStrategy::Parachain,
					sync.clone(),
					pubsub_notification_sinks.clone(),
				),
			);
		},
	}

	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(filter_pool) = filter_pool {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			Some("frontier"),
			EthTask::filter_pool_task(client.clone(), filter_pool, FILTER_RETAIN_THRESHOLD),
		);
	}

	// Spawn Frontier FeeHistory cache maintenance task.
	task_manager.spawn_essential_handle().spawn(
		"frontier-fee-history",
		Some("frontier"),
		EthTask::fee_history_task(
			client.clone(),
			storage_override.clone(),
			fee_history_cache,
			fee_history_cache_limit,
		),
	);

	let _ = config.rpc_addr.as_mut().map(|addr| addr.set_port(rpc_port));
	config.prometheus_config = prometheus_config;

	Ok(config)
}

pub struct PartialComponents {
	pub filter_pool: Option<FilterPool>,
	pub fee_history_cache: FeeHistoryCache,
	pub fee_history_cache_limit: FeeHistoryCacheLimit,
}

pub fn new_partial(config: &Configuration) -> Result<PartialComponents, sc_service::Error> {
	Ok(PartialComponents {
		filter_pool: Some(Arc::new(Mutex::new(BTreeMap::new()))),
		fee_history_cache: Arc::new(Mutex::new(BTreeMap::new())),
		fee_history_cache_limit: config.fee_history_limit,
	})
}
