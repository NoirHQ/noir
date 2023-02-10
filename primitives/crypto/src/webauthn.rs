// This file is part of Noir.

// Copyright (C) 2023 Haderech Pte. Ltd.
// Copyright (C) 2017-2022 Parity Technologies (UK) Ltd.
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

//! Simple WebAuthn API.

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime_interface::runtime_interface;

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Encode, Decode, Debug)]
struct ClientDataJson {
	#[serde(alias = "type")]
	pub type_: String,
	pub challenge: String,
	pub origin: String,
}

#[cfg(feature = "std")]
impl ClientDataJson {
	pub fn verify_challenge(&self, msg: &[u8]) -> bool {
		let challenge = match base64_url::decode(self.challenge.as_str()) {
			Ok(challenge) => challenge,
			Err(_) => return false,
		};
		let msg_hash = sp_io::hashing::sha2_256(msg).to_vec();
		msg_hash == challenge
	}

	pub fn verify_origin(&self, authenticator_data: &[u8]) -> bool {
		let rp_id = match self.origin.starts_with("https://") {
			true => &self.origin["https://".len()..],
			false => return false,
		};
		let rp_id_hash = sp_io::hashing::sha2_256(rp_id.as_bytes()).to_vec();
		rp_id_hash == *&authenticator_data[0..32].to_vec()
	}
}

#[runtime_interface]
pub trait Crypto {
	fn webauthn_es256_verify(
		sig: &crate::p256::Signature,
		msg: &[u8],
		client_data_json: &[u8],
		authenticator_data: &[u8],
		pub_key: &[u8],
	) -> bool {
		let client_data = match serde_json::from_slice::<ClientDataJson>(client_data_json) {
			Ok(client_data) => client_data,
			Err(_) => return false,
		};
		if !client_data.verify_challenge(msg) {
			return false
		}
		if !client_data.verify_origin(authenticator_data) {
			return false
		}

		let mut signed_message: Vec<u8> = Vec::new();
		signed_message.extend_from_slice(authenticator_data);
		let client_data_hash = sp_io::hashing::sha2_256(client_data_json);
		signed_message.extend_from_slice(client_data_hash.as_ref());
		let signed_message = sp_io::hashing::sha2_256(signed_message.as_slice());
		let public = crate::p256::Public::from_full(pub_key).unwrap();
		crate::p256::Pair::verify_prehashed(sig, &signed_message, &public)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_client_data_json_test() {
		let client_data_json = "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiT212TVJpd0dPMW5oZy1WdXBrUUZ0bVJybUp6MEUwdTlnNXc1RWtpYnctVEhmYkJ4dUx4ek5PMVgtVmd6cnVWNDloQ3ZyMGIxT2NuQnJBOUVfaWFhREEiLCJvcmlnaW4iOiJodHRwczovL3dlYmF1dGhuLmlvIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ";
		let client_data_json = base64_url::decode(client_data_json).unwrap();
		let client_data: ClientDataJson =
			serde_json::from_slice(client_data_json.as_slice()).unwrap();

		assert_eq!(client_data.type_, "webauthn.get".to_string());
		assert_eq!(client_data.challenge, "OmvMRiwGO1nhg-VupkQFtmRrmJz0E0u9g5w5Ekibw-THfbBxuLxzNO1X-VgzruV49hCvr0b1OcnBrA9E_iaaDA".to_string());
		assert_eq!(client_data.origin, "https://webauthn.io".to_string());
	}

	#[test]
	fn verify_origin_test() {
		let client_data_json = "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiT212TVJpd0dPMW5oZy1WdXBrUUZ0bVJybUp6MEUwdTlnNXc1RWtpYnctVEhmYkJ4dUx4ek5PMVgtVmd6cnVWNDloQ3ZyMGIxT2NuQnJBOUVfaWFhREEiLCJvcmlnaW4iOiJodHRwczovL3dlYmF1dGhuLmlvIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ";
		let client_data_json = base64_url::decode(client_data_json).unwrap();
		let client_data: ClientDataJson =
			serde_json::from_slice(client_data_json.as_slice()).unwrap();

		let authenticator_data = "dKbqkhPJnC90siSSsyDPQCYqlMGpUKA5fyklC2CEHvABAAAAAg";
		let authenticator_data = base64_url::decode(authenticator_data).unwrap();
		assert!(client_data.verify_origin(authenticator_data.as_slice()));
	}
}
