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

use super::{Error, Gas};

pub trait GasMeter {
	fn new(limit: Gas) -> Self;
	fn consumed_gas(&self) -> Gas;
	fn gas_remaining(&self) -> Gas;
	fn limit(&self) -> Gas;
	fn consume_gas(&mut self, amount: Gas, descriptor: &str) -> Result<Gas, Error>;
}
