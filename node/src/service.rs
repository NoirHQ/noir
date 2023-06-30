// This file is part of Noir.

// Copyright (C) 2023 Haderech Pte. Ltd.
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

//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use crate::{
	cli::Sealing,
	client::{BaseRuntimeApiCollection, FullBackend, FullClient, RuntimeApiCollection},
	eth::{
		new_frontier_partial, spawn_frontier_tasks, FrontierBackend, FrontierBlockImport,
		FrontierPartialComponents,
	},
	rpc,
};
pub use crate::{
	client::{Client, TemplateRuntimeExecutor},
	eth::{db_config_dir, EthConfiguration},
};
use futures::{channel::mpsc, prelude::*};
use noir_core_primitives::Block;
use noir_runtime::{Hash, TransactionConverter};
use prometheus_endpoint::Registry;
use sc_client_api::{BlockBackend, StateBackendFor};
use sc_consensus::BasicQueue;
use sc_executor::{NativeElseWasmExecutor, NativeExecutionDispatch};
use sc_service::{
	error::Error as ServiceError, Configuration, PartialComponents, TaskManager, WarpSyncParams,
};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker};
use sp_api::{ConstructRuntimeApi, TransactionFor};
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_core::U256;
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use std::{cell::RefCell, sync::Arc, time::Duration};

type BasicImportQueue<Client> = sc_consensus::DefaultImportQueue<Block, Client>;
type BoxBlockImport<Client> = sc_consensus::BoxBlockImport<Block, TransactionFor<Client, Block>>;
type FullPool<Client> = sc_transaction_pool::FullPool<Block, Client>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type GrandpaBlockImport<Client> =
	sc_finality_grandpa::GrandpaBlockImport<FullBackend, Block, Client, FullSelectChain>;
type GrandpaLinkHalf<Client> = sc_finality_grandpa::LinkHalf<Block, Client, FullSelectChain>;

pub fn new_partial<RuntimeApi, Executor, BuildImportQueue>(
	config: &Configuration,
	eth_config: &EthConfiguration,
	build_import_queue: BuildImportQueue,
) -> Result<
	PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		FullSelectChain,
		BasicImportQueue<FullClient<RuntimeApi, Executor>>,
		FullPool<FullClient<RuntimeApi, Executor>>,
		(
			Option<Telemetry>,
			BoxBlockImport<FullClient<RuntimeApi, Executor>>,
			GrandpaLinkHalf<FullClient<RuntimeApi, Executor>>,
			Arc<FrontierBackend>,
		),
	>,
	ServiceError,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi: Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		BaseRuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
	BuildImportQueue: FnOnce(
		Arc<FullClient<RuntimeApi, Executor>>,
		&Configuration,
		&EthConfiguration,
		&TaskManager,
		Option<TelemetryHandle>,
		GrandpaBlockImport<FullClient<RuntimeApi, Executor>>,
		Arc<FrontierBackend>,
	) -> Result<
		(
			BasicImportQueue<FullClient<RuntimeApi, Executor>>,
			BoxBlockImport<FullClient<RuntimeApi, Executor>>,
		),
		ServiceError,
	>,
{
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = NativeElseWasmExecutor::<Executor>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
		config.runtime_cache_size,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let frontier_backend =
		Arc::new(FrontierBackend::open(client.clone(), &config.database, &db_config_dir(config))?);
	let (import_queue, block_import) = build_import_queue(
		client.clone(),
		config,
		eth_config,
		&task_manager,
		telemetry.as_ref().map(|x| x.handle()),
		grandpa_block_import,
		frontier_backend.clone(),
	)?;

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	Ok(PartialComponents {
		client,
		backend,
		keystore_container,
		task_manager,
		select_chain,
		import_queue,
		transaction_pool,
		other: (telemetry, block_import, grandpa_link, frontier_backend),
	})
}

/// Build the import queue for the template runtime (aura + grandpa).
pub fn build_aura_grandpa_import_queue<RuntimeApi, Executor>(
	client: Arc<FullClient<RuntimeApi, Executor>>,
	config: &Configuration,
	eth_config: &EthConfiguration,
	task_manager: &TaskManager,
	telemetry: Option<TelemetryHandle>,
	grandpa_block_import: GrandpaBlockImport<FullClient<RuntimeApi, Executor>>,
	frontier_backend: Arc<FrontierBackend>,
) -> Result<
	(
		BasicImportQueue<FullClient<RuntimeApi, Executor>>,
		BoxBlockImport<FullClient<RuntimeApi, Executor>>,
	),
	ServiceError,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi: Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
	let frontier_block_import =
		FrontierBlockImport::new(grandpa_block_import.clone(), client.clone(), frontier_backend);

	let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
	let target_gas_price = eth_config.target_gas_price;
	let create_inherent_data_providers = move |_, ()| async move {
		let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
		let slot =
			sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
				*timestamp,
				slot_duration,
			);
		let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
		Ok((slot, timestamp, dynamic_fee))
	};

	let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
		sc_consensus_aura::ImportQueueParams {
			block_import: frontier_block_import.clone(),
			justification_import: Some(Box::new(grandpa_block_import)),
			client,
			create_inherent_data_providers,
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			check_for_equivocation: Default::default(),
			telemetry,
			compatibility_mode: sc_consensus_aura::CompatibilityMode::None,
		},
	)
	.map_err::<ServiceError, _>(Into::into)?;

	Ok((import_queue, Box::new(frontier_block_import)))
}

/// Build the import queue for the template runtime (manual seal).
pub fn build_manual_seal_import_queue<RuntimeApi, Executor>(
	client: Arc<FullClient<RuntimeApi, Executor>>,
	config: &Configuration,
	_eth_config: &EthConfiguration,
	task_manager: &TaskManager,
	_telemetry: Option<TelemetryHandle>,
	_grandpa_block_import: GrandpaBlockImport<FullClient<RuntimeApi, Executor>>,
	frontier_backend: Arc<FrontierBackend>,
) -> Result<
	(
		BasicImportQueue<FullClient<RuntimeApi, Executor>>,
		BoxBlockImport<FullClient<RuntimeApi, Executor>>,
	),
	ServiceError,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi: Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
	let frontier_block_import = FrontierBlockImport::new(client.clone(), client, frontier_backend);
	Ok((
		sc_consensus_manual_seal::import_queue(
			Box::new(frontier_block_import.clone()),
			&task_manager.spawn_essential_handle(),
			config.prometheus_registry(),
		),
		Box::new(frontier_block_import),
	))
}

fn run_manual_seal_authorship<RuntimeApi, Executor>(
	eth_config: &EthConfiguration,
	sealing: Sealing,
	client: Arc<FullClient<RuntimeApi, Executor>>,
	transaction_pool: Arc<FullPool<FullClient<RuntimeApi, Executor>>>,
	select_chain: FullSelectChain,
	block_import: BoxBlockImport<FullClient<RuntimeApi, Executor>>,
	task_manager: &TaskManager,
	prometheus_registry: Option<&Registry>,
	telemetry: Option<&Telemetry>,
	commands_stream: mpsc::Receiver<sc_consensus_manual_seal::rpc::EngineCommand<Hash>>,
) -> Result<(), ServiceError>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi: Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
	let proposer_factory = sc_basic_authorship::ProposerFactory::new(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool.clone(),
		prometheus_registry,
		telemetry.as_ref().map(|x| x.handle()),
	);

	thread_local!(static TIMESTAMP: RefCell<u64> = RefCell::new(0));

	/// Provide a mock duration starting at 0 in millisecond for timestamp inherent.
	/// Each call will increment timestamp by slot_duration making Aura think time has passed.
	struct MockTimestampInherentDataProvider;

	#[async_trait::async_trait]
	impl sp_inherents::InherentDataProvider for MockTimestampInherentDataProvider {
		async fn provide_inherent_data(
			&self,
			inherent_data: &mut sp_inherents::InherentData,
		) -> Result<(), sp_inherents::Error> {
			TIMESTAMP.with(|x| {
				*x.borrow_mut() += noir_runtime::SLOT_DURATION;
				inherent_data.put_data(sp_timestamp::INHERENT_IDENTIFIER, &*x.borrow())
			})
		}

		async fn try_handle_error(
			&self,
			_identifier: &sp_inherents::InherentIdentifier,
			_error: &[u8],
		) -> Option<Result<(), sp_inherents::Error>> {
			// The pallet never reports error.
			None
		}
	}

	let target_gas_price = eth_config.target_gas_price;
	let create_inherent_data_providers = move |_, ()| async move {
		let timestamp = MockTimestampInherentDataProvider;
		let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
		Ok((timestamp, dynamic_fee))
	};

	let manual_seal = match sealing {
		Sealing::Manual => future::Either::Left(sc_consensus_manual_seal::run_manual_seal(
			sc_consensus_manual_seal::ManualSealParams {
				block_import,
				env: proposer_factory,
				client,
				pool: transaction_pool,
				commands_stream,
				select_chain,
				consensus_data_provider: None,
				create_inherent_data_providers,
			},
		)),
		Sealing::Instant => future::Either::Right(sc_consensus_manual_seal::run_instant_seal(
			sc_consensus_manual_seal::InstantSealParams {
				block_import,
				env: proposer_factory,
				client,
				pool: transaction_pool,
				select_chain,
				consensus_data_provider: None,
				create_inherent_data_providers,
			},
		)),
	};

	// we spawn the future on a background thread managed by service.
	task_manager
		.spawn_essential_handle()
		.spawn_blocking("manual-seal", None, manual_seal);
	Ok(())
}

/// Builds a new service for a full client.
pub fn new_full<RuntimeApi, Executor>(
	mut config: Configuration,
	eth_config: EthConfiguration,
	sealing: Option<Sealing>,
) -> Result<TaskManager, ServiceError>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi: Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
	let build_import_queue = if sealing.is_some() {
		build_manual_seal_import_queue::<RuntimeApi, Executor>
	} else {
		build_aura_grandpa_import_queue::<RuntimeApi, Executor>
	};

	let PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (mut telemetry, block_import, grandpa_link, frontier_backend),
	} = new_partial(&config, &eth_config, build_import_queue)?;

	let FrontierPartialComponents { filter_pool, fee_history_cache, fee_history_cache_limit } =
		new_frontier_partial(&eth_config)?;

	let grandpa_protocol_name = sc_finality_grandpa::protocol_standard_name(
		&client.block_hash(0)?.expect("Genesis block exists; qed"),
		&config.chain_spec,
	);

	config
		.network
		.extra_sets
		.push(sc_finality_grandpa::grandpa_peers_set_config(grandpa_protocol_name.clone()));

	let warp_sync = Arc::new(sc_finality_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		grandpa_link.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_params: if sealing.is_some() {
				None
			} else {
				Some(WarpSyncParams::WithProvider(warp_sync))
			},
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa && sealing.is_none();
	let prometheus_registry = config.prometheus_registry().cloned();

	// Channel for the rpc handler to communicate with the authorship task.
	let (command_sink, commands_stream) = mpsc::channel(1000);

	// for ethereum-compatibility rpc.
	config.rpc_id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));
	let overrides = rpc::overrides_handle(client.clone());
	let eth_rpc_params = rpc::EthDeps {
		client: client.clone(),
		pool: transaction_pool.clone(),
		graph: transaction_pool.pool().clone(),
		converter: Some(TransactionConverter),
		is_authority: config.role.is_authority(),
		enable_dev_signer: eth_config.enable_dev_signer,
		network: network.clone(),
		frontier_backend: frontier_backend.clone(),
		overrides: overrides.clone(),
		block_data_cache: Arc::new(fc_rpc::EthBlockDataCacheTask::new(
			task_manager.spawn_handle(),
			overrides.clone(),
			eth_config.eth_log_block_cache,
			eth_config.eth_statuses_cache,
			prometheus_registry.clone(),
		)),
		filter_pool: filter_pool.clone(),
		max_past_logs: eth_config.max_past_logs,
		fee_history_cache: fee_history_cache.clone(),
		fee_history_cache_limit,
		execute_gas_limit_multiplier: eth_config.execute_gas_limit_multiplier,
	};

	let rpc_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let chain_spec = config.chain_spec.cloned_box();

		Box::new(move |deny_unsafe, subscription_task_executor| {
			let deps = rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				command_sink: if sealing.is_some() { Some(command_sink.clone()) } else { None },
				eth: eth_rpc_params.clone(),
				chain_spec: chain_spec.cloned_box(),
			};

			rpc::create_full(deps, subscription_task_executor).map_err(Into::into)
		})
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		client: client.clone(),
		backend: backend.clone(),
		task_manager: &mut task_manager,
		keystore: keystore_container.sync_keystore(),
		transaction_pool: transaction_pool.clone(),
		rpc_builder,
		network: network.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
	})?;

	spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend,
		frontier_backend,
		filter_pool,
		overrides,
		fee_history_cache,
		fee_history_cache_limit,
	);

	if role.is_authority() {
		// manual-seal authorship
		if let Some(sealing) = sealing {
			run_manual_seal_authorship(
				&eth_config,
				sealing,
				client,
				transaction_pool,
				select_chain,
				block_import,
				&task_manager,
				prometheus_registry.as_ref(),
				telemetry.as_ref(),
				commands_stream,
			)?;

			network_starter.start_network();
			log::info!("Manual Seal Ready");
			return Ok(task_manager);
		}

		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
		let target_gas_price = eth_config.target_gas_price;
		let create_inherent_data_providers = move |_, ()| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
				*timestamp,
				slot_duration,
			);
			let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
			Ok((slot, timestamp, dynamic_fee))
		};

		let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
			sc_consensus_aura::StartAuraParams {
				slot_duration,
				client,
				select_chain,
				block_import,
				proposer_factory,
				sync_oracle: network.clone(),
				justification_sync_link: network.clone(),
				create_inherent_data_providers,
				force_authoring,
				backoff_authoring_blocks: Option::<()>::None,
				keystore: keystore_container.sync_keystore(),
				block_proposal_slot_portion: sc_consensus_aura::SlotProportion::new(2f32 / 3f32),
				max_block_proposal_slot_portion: None,
				telemetry: telemetry.as_ref().map(|x| x.handle()),
				compatibility_mode: sc_consensus_aura::CompatibilityMode::None,
			},
		)?;
		// the AURA authoring task is considered essential, i.e. if it
		// fails we take down the service with it.
		task_manager
			.spawn_essential_handle()
			.spawn_blocking("aura", Some("block-authoring"), aura);
	}

	if enable_grandpa {
		// if the node isn't actively participating in consensus then it doesn't
		// need a keystore, regardless of which protocol we use below.
		let keystore =
			if role.is_authority() { Some(keystore_container.sync_keystore()) } else { None };

		let grandpa_config = sc_finality_grandpa::Config {
			// FIXME #1578 make this available through chainspec
			gossip_duration: Duration::from_millis(333),
			justification_period: 512,
			name: Some(name),
			observer_enabled: false,
			keystore,
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: grandpa_protocol_name,
		};

		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_voter =
			sc_finality_grandpa::run_grandpa_voter(sc_finality_grandpa::GrandpaParams {
				config: grandpa_config,
				link: grandpa_link,
				network,
				voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
				prometheus_registry,
				shared_voter_state: sc_finality_grandpa::SharedVoterState::empty(),
				telemetry: telemetry.as_ref().map(|x| x.handle()),
			})?;

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager
			.spawn_essential_handle()
			.spawn_blocking("grandpa-voter", None, grandpa_voter);
	}

	network_starter.start_network();
	Ok(task_manager)
}

pub fn build_full(
	config: Configuration,
	eth_config: EthConfiguration,
	sealing: Option<Sealing>,
) -> Result<TaskManager, ServiceError> {
	new_full::<noir_runtime::RuntimeApi, TemplateRuntimeExecutor>(config, eth_config, sealing)
}

pub fn new_chain_ops(
	config: &mut Configuration,
	eth_config: &EthConfiguration,
) -> Result<
	(
		Arc<Client>,
		Arc<FullBackend>,
		BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
		Arc<FrontierBackend>,
	),
	ServiceError,
> {
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	let PartialComponents { client, backend, import_queue, task_manager, other, .. } =
		new_partial::<noir_runtime::RuntimeApi, TemplateRuntimeExecutor, _>(
			config,
			eth_config,
			build_aura_grandpa_import_queue,
		)?;
	Ok((client, backend, import_queue, task_manager, other.3))
}
