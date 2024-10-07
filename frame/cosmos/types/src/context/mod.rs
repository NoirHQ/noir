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

pub mod traits;

use crate::{
	events::{traits::EventManager as _, EventManager},
	gas::{traits::GasMeter, BasicGasMeter, Gas},
};

pub struct Context {
	pub gas_meter: BasicGasMeter,
	pub event_manager: EventManager,
}

impl traits::Context for Context {
	type GasMeter = BasicGasMeter;
	type EventManager = EventManager;

	fn new(limit: Gas) -> Self {
		Self { gas_meter: Self::GasMeter::new(limit), event_manager: Self::EventManager::new() }
	}

	fn gas_meter(&mut self) -> &mut Self::GasMeter {
		&mut self.gas_meter
	}

	fn event_manager(&mut self) -> &mut Self::EventManager {
		&mut self.event_manager
	}
}
