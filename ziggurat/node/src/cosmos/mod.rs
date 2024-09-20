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

pub mod rpc;

use jsonrpsee::{core::id_providers::RandomIntegerIdProvider, RpcModule};
use sc_rpc::DenyUnsafe;
use sc_service::{Error as ServiceError, TaskManager};

pub struct SpawnTasksParams<'a> {
	pub config: sc_service::Configuration,
	pub rpc_builder: Box<dyn Fn(DenyUnsafe) -> Result<RpcModule<()>, ServiceError>>,
	pub task_manager: &'a mut TaskManager,
}

pub fn spawn_tasks(
	params: SpawnTasksParams,
) -> Result<sc_service::Configuration, sc_service::Error> {
	let SpawnTasksParams { mut config, rpc_builder, task_manager } = params;

	let rpc_port = config.rpc_addr.map(|addr| addr.port()).unwrap_or(config.rpc_port);
	let prometheus_config = config.prometheus_config.take();

	// TODO: Make the Cosmos RPC port configurable.
	let _ = config.rpc_addr.as_mut().map(|addr| addr.set_port(26657));

	let rpc = sc_service::start_rpc_servers(
		&config,
		rpc_builder,
		Some(Box::new(RandomIntegerIdProvider)),
	);
	if rpc.is_ok() {
		log::info!("Cosmos RPC started: {}", config.rpc_port);
	} else {
		log::warn!("Cosmos RPC not started");
	}
	task_manager.keep_alive(rpc);

	let _ = config.rpc_addr.as_mut().map(|addr| addr.set_port(rpc_port));
	config.prometheus_config = prometheus_config;

	Ok(config)
}
