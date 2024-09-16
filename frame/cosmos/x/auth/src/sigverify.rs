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

use core::{cmp::Ordering, marker::PhantomData};
use cosmos_sdk_proto::{
	cosmos::{
		crypto::{multisig::LegacyAminoPubKey, secp256k1},
		tx::v1beta1::{ModeInfo, SignerInfo, Tx},
	},
	prost::Message,
	Any,
};
use frame_support::{
	ensure,
	pallet_prelude::{InvalidTransaction, TransactionValidity, ValidTransaction},
};
use np_cosmos::traits::ChainInfo;
use pallet_cosmos::AddressMapping;
use pallet_cosmos_types::{address::acc_address_from_bech32, any_match, handler::AnteDecorator};
use pallet_cosmos_x_auth_signing::{
	sign_mode_handler::{traits::SignModeHandler, SignerData},
	sign_verifiable_tx::traits::SigVerifiableTx,
};
use ripemd::Digest;
use sp_core::{ecdsa, sha2_256, ByteArray, Get, H160};
use sp_runtime::{transaction_validity::TransactionValidityError, SaturatedConversion};

pub struct SigVerificationDecorator<T>(PhantomData<T>);

impl<T> AnteDecorator for SigVerificationDecorator<T>
where
	T: pallet_cosmos::Config + frame_system::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> TransactionValidity {
		let signatures = &tx.signatures;
		let signers = T::SigVerifiableTx::get_signers(tx).map_err(|_| InvalidTransaction::Call)?;
		let signer_infos = &tx.auth_info.as_ref().ok_or(InvalidTransaction::Call)?.signer_infos;

		ensure!(signatures.len() == signers.len(), InvalidTransaction::Call);
		ensure!(signatures.len() == signer_infos.len(), InvalidTransaction::Call);

		for (i, sig) in signatures.iter().enumerate() {
			let signer = signers.get(i).ok_or(InvalidTransaction::Call)?;
			let signer_info = signer_infos.get(i).ok_or(InvalidTransaction::Call)?;

			let (_hrp, signer_addr_raw) =
				acc_address_from_bech32(signer).map_err(|_| InvalidTransaction::BadSigner)?;
			ensure!(signer_addr_raw.len() == 20, InvalidTransaction::BadSigner);

			let who = T::AddressMapping::into_account_id(H160::from_slice(&signer_addr_raw));
			let sequence = frame_system::Pallet::<T>::account_nonce(&who).saturated_into();

			match signer_info.sequence.cmp(&sequence) {
				Ordering::Greater => Err(InvalidTransaction::Future),
				Ordering::Less => Err(InvalidTransaction::Stale),
				Ordering::Equal => Ok(()),
			}?;

			let public_key =
				signer_info.public_key.as_ref().ok_or(InvalidTransaction::Call)?;
			let chain_id = T::ChainInfo::chain_id().to_string();
			let signer_data = SignerData {
				address: signer.clone(),
				chain_id,
				account_number: 0,
				sequence: signer_info.sequence,
				pub_key: public_key.clone(),
			};
			let sign_mode = signer_info.mode_info.as_ref().ok_or(InvalidTransaction::Call)?;

			Self::verify_signature(public_key, &signer_data, sign_mode, sig, tx)?;
		}

		Ok(ValidTransaction::default())
	}
}

impl<T> SigVerificationDecorator<T>
where
	T: pallet_cosmos::Config,
{
	fn verify_signature(
		public_key: &Any,
		signer_data: &SignerData,
		sign_mode: &ModeInfo,
		signature: &[u8],
		tx: &Tx,
	) -> Result<(), TransactionValidityError> {
		any_match!(
			public_key, {
				secp256k1::PubKey => {
					let public_key =
						secp256k1::PubKey::decode(&mut &*public_key.value).map_err(|_| {
							InvalidTransaction::BadSigner
						})?;

					let mut hasher = ripemd::Ripemd160::new();
					hasher.update(sha2_256(&public_key.key));
					let address = H160::from_slice(&hasher.finalize());

					let (_hrp, signer_addr_raw) =
						acc_address_from_bech32(&signer_data.address).map_err(|_| {
							InvalidTransaction::BadSigner
						})?;
					ensure!(signer_addr_raw.len() == 20, InvalidTransaction::BadSigner);

					ensure!(H160::from_slice(&signer_addr_raw) == address, InvalidTransaction::BadSigner);

					let sign_bytes = T::SignModeHandler::get_sign_bytes(sign_mode, signer_data, tx)
						.map_err(|_| InvalidTransaction::Call)?;

					if !ecdsa_verify_prehashed(signature, &sign_bytes, &public_key.key) {
						return Err(InvalidTransaction::BadProof.into());
					}

					Ok(())
				}
			},
			Err(InvalidTransaction::BadSigner.into())
		)
	}
}

pub fn ecdsa_verify_prehashed(signature: &[u8], message: &[u8], public_key: &[u8]) -> bool {
	let pub_key = match ecdsa::Public::from_slice(public_key) {
		Ok(pub_key) => pub_key,
		Err(_) => return false,
	};
	let msg = sha2_256(message);

	match signature.len() {
		64 => (0..=3).any(|rec_id| {
			let mut rec_sig = [0u8; 65];
			rec_sig[..64].copy_from_slice(signature);
			rec_sig[64] = rec_id;
			let sig = ecdsa::Signature::from(rec_sig);
			sp_io::crypto::ecdsa_verify_prehashed(&sig, &msg, &pub_key)
		}),
		65 => ecdsa::Signature::try_from(signature)
			.map_or(false, |sig| sp_io::crypto::ecdsa_verify_prehashed(&sig, &msg, &pub_key)),
		_ => false,
	}
}

#[cfg(test)]
pub mod tests {
	use super::*;

	#[test]
	fn ecdsa_verify_test() {
		let sig = hex::decode("f7e0d198c62821cc5817c8e935f523308301e29819f5d882f3249b9e173a614f38000ddbff446c0abfa7c7d019dbb17072b28933fc8187c973fbf03d0459f76e").unwrap();
		let message = hex::decode("0a93010a90010a1c2f636f736d6f732e62616e6b2e763162657461312e4d736753656e6412700a2d636f736d6f7331716436396e75776a393567746134616b6a677978746a39756a6d7a34773865646d7179737177122d636f736d6f7331676d6a32657861673033747467616670726b6463337438383067726d61396e776566636432771a100a057561746f6d12073130303030303012710a4e0a460a1f2f636f736d6f732e63727970746f2e736563703235366b312e5075624b657912230a21020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a112040a020801121f0a150a057561746f6d120c3838363838303030303030301080c0f1c59495141a1174686574612d746573746e65742d30303120ad8a2e").unwrap();
		let public_key =
			hex::decode("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1")
				.unwrap();

		assert!(ecdsa_verify_prehashed(&sig, &message, &public_key));
	}
}

pub struct ValidateSigCountDecorator<T>(PhantomData<T>);

impl<T> AnteDecorator for ValidateSigCountDecorator<T>
where
	T: pallet_cosmos::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> TransactionValidity {
		let mut sig_count = 0u64;

		let auth_info = tx.auth_info.as_ref().ok_or(InvalidTransaction::Call)?;
		for SignerInfo { public_key, .. } in auth_info.signer_infos.iter() {
			let public_key = public_key.as_ref().ok_or(InvalidTransaction::BadSigner)?;
			sig_count = sig_count.saturating_add(Self::count_sub_keys(public_key)?);

			ensure!(sig_count <= T::TxSigLimit::get(), InvalidTransaction::Call);
		}

		Ok(ValidTransaction::default())
	}
}

impl<T> ValidateSigCountDecorator<T> {
	fn count_sub_keys(pubkey: &Any) -> Result<u64, TransactionValidityError> {
		// TODO: Support legacy multi signatures.
		if LegacyAminoPubKey::decode(&mut &*pubkey.value).is_ok() {
			Err(InvalidTransaction::BadSigner.into())
		} else {
			Ok(1)
		}
	}
}

pub struct IncrementSequenceDecorator<T>(PhantomData<T>);

impl<T> AnteDecorator for IncrementSequenceDecorator<T>
where
	T: frame_system::Config + pallet_cosmos::Config,
{
	fn ante_handle(tx: &Tx, _simulate: bool) -> TransactionValidity {
		let signers = T::SigVerifiableTx::get_signers(tx).map_err(|_| InvalidTransaction::Call)?;
		for signer in signers.iter() {
			let (_hrp, address_raw) =
				acc_address_from_bech32(signer).map_err(|_| InvalidTransaction::BadSigner)?;
			ensure!(address_raw.len() == 20, InvalidTransaction::BadSigner);

			let account = T::AddressMapping::into_account_id(H160::from_slice(&address_raw));
			frame_system::pallet::Pallet::<T>::inc_account_nonce(account);
		}

		Ok(ValidTransaction::default())
	}
}
