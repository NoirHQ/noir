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

use {
    nostd::{collections::HashMap, fmt, time::Duration},
    solana_sdk::pubkey::Pubkey,
};

#[derive(Debug)]
pub struct Measure;

impl Measure {
    pub fn start(_name: &'static str) -> Self {
        Self {}
    }

    pub fn stop(&mut self) {}

    pub fn as_ns(&self) -> u64 {
        0
    }

    pub fn as_us(&self) -> u64 {
        0
    }

    pub fn as_ms(&self) -> u64 {
        0
    }

    pub fn as_s(&self) -> f32 {
        0.0
    }

    pub fn as_duration(&self) -> Duration {
        Duration::from_nanos(0)
    }

    pub fn end_as_ns(self) -> u64 {
        0
    }

    pub fn end_as_us(self) -> u64 {
        0
    }

    pub fn end_as_ms(self) -> u64 {
        0
    }

    pub fn end_as_s(self) -> f32 {
        0.0
    }

    pub fn end_as_duration(self) -> Duration {
        Duration::from_nanos(0)
    }
}

impl fmt::Display for Measure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Measure")
    }
}

pub type VoteAccountsHashMap = HashMap<Pubkey, (u64, VoteAccount)>;

pub struct VoteAccount;
