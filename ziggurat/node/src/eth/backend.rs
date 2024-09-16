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

use fc_db::Backend;
use fc_rpc::StorageOverrideHandler;
use fp_rpc::EthereumRuntimeRPCApi;
use sc_client_api::{HeaderBackend, StorageProvider};
use sc_service::Configuration;
use sp_api::ProvideRuntimeApi;
use std::{
	path::{Path, PathBuf},
	sync::Arc,
};

use crate::{eth::FrontierBackendType, service::ParachainBackend};

pub fn db_config_dir(config: &Configuration) -> PathBuf {
	config.base_path.config_dir(config.chain_spec.id())
}

pub fn open_frontier_backend<C>(
	client: Arc<C>,
	config: &Configuration,
	eth_config: &super::Configuration,
) -> Result<fc_db::Backend<Block, C>, String>
where
	C: HeaderBackend<Block>
		+ ProvideRuntimeApi<Block>
		+ StorageProvider<Block, ParachainBackend>
		+ 'static,
	C::Api: EthereumRuntimeRPCApi<Block>,
{
	Ok(match eth_config.frontier_backend_type {
		FrontierBackendType::KeyValue => Backend::KeyValue(Arc::new(fc_db::kv::Backend::open(
			client.clone(),
			&config.database,
			&db_config_dir(config),
		)?)),
		FrontierBackendType::Sql => {
			let overrides = Arc::new(StorageOverrideHandler::new(client.clone()));
			let db_path = db_config_dir(config).join("sql");
			std::fs::create_dir_all(&db_path).expect("failed creating sql db directory");
			let backend = futures::executor::block_on(fc_db::sql::Backend::new(
				fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
					path: Path::new("sqlite:///")
						.join(db_path)
						.join("frontier.db3")
						.to_str()
						.unwrap(),
					create_if_missing: true,
					thread_count: eth_config.frontier_sql_backend_thread_count,
					cache_size: eth_config.frontier_sql_backend_cache_size,
				}),
				eth_config.frontier_sql_backend_pool_size,
				std::num::NonZeroU32::new(eth_config.frontier_sql_backend_num_ops_timeout),
				overrides,
			))
			.unwrap_or_else(|err| panic!("failed creating sql backend: {:?}", err));
			Backend::Sql(Arc::new(backend))
		},
	})
}
