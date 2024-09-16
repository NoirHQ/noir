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

use cumulus_primitives_core::PersistedValidationData;
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use fc_rpc::{EthBlockDataCacheTask, StorageOverride};
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fp_rpc::{ConvertTransactionRuntimeApi, EthereumRuntimeRPCApi};
use jsonrpsee::RpcModule;
use sc_client_api::{
	client::BlockchainEvents, AuxStore, Backend, HeaderBackend, StorageProvider, UsageProvider,
};
use sc_network::service::traits::NetworkService;
use sc_network_sync::SyncingService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool::{ChainApi, Pool};
use sc_transaction_pool_api::TransactionPool;
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_consensus_aura::{sr25519::AuthorityId as AuraId, AuraApi};
use sp_core::H256;
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

pub struct EthConfig<C, BE>(PhantomData<(C, BE)>);

impl<C, BE> fc_rpc::EthConfig<Block, C> for EthConfig<C, BE>
where
	C: StorageProvider<Block, BE> + Send + Sync + 'static,
	BE: Backend<Block> + 'static,
{
	type EstimateGasAdapter = ();
	type RuntimeStorageOverride =
		fc_rpc::frontier_backend_client::SystemAccountId20StorageOverride<Block, C, BE>;
}

/// Extra dependencies for Ethereum compatibility.
pub struct FullDeps<C, P, A: ChainApi> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	/// The Node authority flag
	pub is_authority: bool,
	/// Network service
	pub network: Arc<dyn NetworkService>,
	/// Chain syncing service
	pub sync: Arc<SyncingService<Block>>,
	/// Frontier Backend.
	pub frontier_backend: Arc<dyn fc_api::Backend<Block>>,
	/// Ethereum data access overrides.
	pub storage_override: Arc<dyn StorageOverride<Block>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
	/// Maximum number of logs in a query.
	pub max_past_logs: u32,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// Maximum fee history cache size.
	pub fee_history_cache_limit: FeeHistoryCacheLimit,
	/// Mandated parent hashes for a given block hash.
	pub forced_parent_hashes: Option<BTreeMap<H256, H256>>,
	///
	pub subscription_task_executor: SubscriptionTaskExecutor,
	///
	pub pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<Block>,
		>,
	>,
}

/// Instantiate Ethereum-compatible RPC extensions.
pub fn create_full<C, P, A, BE>(
	deps: FullDeps<C, P, A>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: CallApiAt<Block>
		+ ProvideRuntimeApi<Block>
		+ AuxStore
		+ StorageProvider<Block, BE>
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockchainEvents<Block>
		+ 'static,
	C::Api: AuraApi<Block, AuraId>
		+ EthereumRuntimeRPCApi<Block>
		+ BlockBuilder<Block>
		+ ConvertTransactionRuntimeApi<Block>,
	P: TransactionPool<Block = Block> + 'static,
	A: ChainApi<Block = Block> + 'static,
	BE: Backend<Block> + 'static,
{
	use fc_rpc::{
		pending::AuraConsensusDataProvider, Debug, DebugApiServer, Eth, EthApiServer, EthFilter,
		EthFilterApiServer, EthPubSub, EthPubSubApiServer, Net, NetApiServer, TxPool,
		TxPoolApiServer, Web3, Web3ApiServer,
	};

	let mut io = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		graph,
		is_authority,
		network,
		sync,
		frontier_backend,
		storage_override,
		block_data_cache,
		filter_pool,
		max_past_logs,
		fee_history_cache,
		fee_history_cache_limit,
		forced_parent_hashes,
		subscription_task_executor,
		pubsub_notification_sinks,
	} = deps;

	let signers = Vec::new();

	enum Never {}
	impl<T> fp_rpc::ConvertTransaction<T> for Never {
		fn convert_transaction(&self, _transaction: pallet_ethereum::Transaction) -> T {
			// The Never type is not instantiable, but this method requires the type to be
			// instantiated to be called (`&self` parameter), so if the code compiles we
			// have the guarantee that this function will never be called.
			unreachable!()
		}
	}
	let convert_transaction: Option<Never> = None;

	let pending_create_inherent_data_providers = move |_, _| async move {
		let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
		// Create a dummy parachain inherent data provider which is required to pass
		// the checks by the para chain system. We use dummy values because in the 'pending
		// context' neither do we have access to the real values nor do we need them.
		let (relay_parent_storage_root, relay_chain_state) =
			RelayStateSproofBuilder::default().into_state_root_and_proof();
		let vfp = PersistedValidationData {
			// This is a hack to make
			// `cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases` happy. Relay
			// parent number can't be bigger than u32::MAX.
			relay_parent_number: u32::MAX,
			relay_parent_storage_root,
			..Default::default()
		};
		let parachain_inherent_data = ParachainInherentData {
			validation_data: vfp,
			relay_chain_state,
			downward_messages: Default::default(),
			horizontal_messages: Default::default(),
		};
		Ok((timestamp, parachain_inherent_data))
	};

	io.merge(
		Eth::<_, _, _, _, _, _, _, EthConfig<_, _>>::new(
			client.clone(),
			pool.clone(),
			graph.clone(),
			convert_transaction,
			sync.clone(),
			signers,
			storage_override.clone(),
			frontier_backend.clone(),
			is_authority,
			block_data_cache.clone(),
			fee_history_cache,
			fee_history_cache_limit,
			10, /* execute_gas_limit_multiplier */
			forced_parent_hashes,
			pending_create_inherent_data_providers,
			Some(Box::new(AuraConsensusDataProvider::new(client.clone()))),
		)
		.replace_config::<EthConfig<_, _>>()
		.into_rpc(),
	)?;

	if let Some(filter_pool) = filter_pool {
		io.merge(
			EthFilter::new(
				client.clone(),
				frontier_backend.clone(),
				graph.clone(),
				filter_pool,
				500_usize, /* max stored filters */
				max_past_logs,
				block_data_cache.clone(),
			)
			.into_rpc(),
		)?;
	}

	io.merge(
		Net::new(
			client.clone(),
			network.clone(),
			// Whether to format the `peer_count` response as Hex (default) or not.
			true,
		)
		.into_rpc(),
	)?;

	io.merge(Web3::new(client.clone()).into_rpc())?;

	io.merge(
		EthPubSub::new(
			pool,
			client.clone(),
			sync.clone(),
			subscription_task_executor,
			storage_override.clone(),
			pubsub_notification_sinks.clone(),
		)
		.into_rpc(),
	)?;

	io.merge(
		Debug::new(client.clone(), frontier_backend, storage_override, block_data_cache).into_rpc(),
	)?;

	io.merge(TxPool::new(client, graph).into_rpc())?;

	Ok(io)
}
