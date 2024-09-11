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

use crate::{
	events::traits::EventManager,
	gas::{traits::GasMeter, Gas},
};

pub trait Context {
	type GasMeter: GasMeter;
	type EventManager: EventManager;

	fn new(limit: Gas) -> Self;
	fn gas_meter(&mut self) -> &mut Self::GasMeter;
	fn event_manager(&mut self) -> &mut Self::EventManager;
}
