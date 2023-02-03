// This file is part of Noir.

// Copyright (C) 2023 Haderech Pte. Ltd.
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

// TODO: add documentation
#![doc(hidden)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::crypto::{
	ByteArray, CryptoType, CryptoTypeId, CryptoTypePublicPair, Derive, Public as TraitPublic,
};
use sp_runtime_interface::pass_by::PassByInner;

#[cfg(feature = "full_crypto")]
use p256::{
	ecdsa::{
		signature::hazmat::{PrehashSigner, PrehashVerifier},
		Signature as EcdsaSignature, SigningKey, VerifyingKey,
	},
	elliptic_curve::sec1::ToEncodedPoint,
	PublicKey,
};
#[cfg(feature = "full_crypto")]
use sp_core::{
	crypto::{DeriveJunction, Pair as TraitPair, SecretStringError},
	hashing::{blake2_256, sha2_256},
};
#[cfg(feature = "full_crypto")]
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use bip39::{Language, Mnemonic, MnemonicType};
#[cfg(feature = "std")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "std")]
use sp_core::crypto::Ss58Codec;

pub const CRYPTO_ID: CryptoTypeId = CryptoTypeId(*b"es25");

#[cfg_attr(feature = "full_crypto", derive(Hash))]
#[derive(
	Copy,
	Clone,
	PartialEq,
	PartialOrd,
	Eq,
	Ord,
	Encode,
	Decode,
	MaxEncodedLen,
	TypeInfo,
	PassByInner,
)]
pub struct Public(pub [u8; 33]);

impl ByteArray for Public {
	const LEN: usize = 33;
}

impl AsRef<[u8]> for Public {
	fn as_ref(&self) -> &[u8] {
		&self.0[..]
	}
}

impl AsMut<[u8]> for Public {
	fn as_mut(&mut self) -> &mut [u8] {
		&mut self.0[..]
	}
}

impl TryFrom<&[u8]> for Public {
	type Error = ();

	#[cfg(feature = "full_crypto")]
	fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
		if data.len() == 64 {
			let mut x = [0u8; 65];
			x[0] = 0x4u8;
			x[1..].copy_from_slice(data);
			Self::try_from(&x[..])
		} else {
			let k = PublicKey::from_sec1_bytes(data).map_err(|_| ())?;
			let x = <[u8; 33]>::try_from(k.to_encoded_point(true).as_bytes()).map_err(|_| ())?;
			Ok(Self(x))
		}
	}

	#[cfg(not(feature = "full_crypto"))]
	fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
		if data.len() != Self::LEN {
			return Err(())
		}
		let mut x = [0u8; 33];
		x.copy_from_slice(data);
		Ok(Public(x))
	}
}

impl Derive for Public {}

impl TraitPublic for Public {
	fn to_public_crypto_pair(&self) -> CryptoTypePublicPair {
		CryptoTypePublicPair(CRYPTO_ID, self.to_raw_vec())
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for Public {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.to_ss58check())
	}
}

impl sp_std::fmt::Debug for Public {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let s = self.to_ss58check();
		write!(f, "{} ({}...)", sp_core::hexdisplay::HexDisplay::from(&self.as_ref()), &s[0..8])
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

impl sp_std::fmt::Debug for Signature {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "{}", sp_core::hexdisplay::HexDisplay::from(&self.0))
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

#[cfg(feature = "std")]
impl Serialize for Public {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.to_ss58check())
	}
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for Public {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Public::from_ss58check(&String::deserialize(deserializer)?)
			.map_err(|e| de::Error::custom(format!("{:?}", e)))
	}
}

#[cfg_attr(feature = "full_crypto", derive(Hash))]
#[derive(PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo, PassByInner)]
pub struct Signature(pub [u8; 64]);

impl AsRef<[u8]> for Signature {
	fn as_ref(&self) -> &[u8] {
		&self.0[..]
	}
}

impl TryFrom<&[u8]> for Signature {
	type Error = ();

	fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
		if data.len() != 64 {
			return Err(())
		}
		let mut x = [0u8; 64];
		x.copy_from_slice(data);
		Ok(Signature(x))
	}
}

#[cfg(feature = "std")]
impl Serialize for Signature {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&array_bytes::bytes2hex("", self.as_ref()))
	}
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for Signature {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let signature_hex = array_bytes::hex2bytes(&String::deserialize(deserializer)?)
			.map_err(|e| de::Error::custom(format!("{:?}", e)))?;
		Signature::try_from(signature_hex.as_ref())
			.map_err(|e| de::Error::custom(format!("{:?}", e)))
	}
}

#[cfg(feature = "full_crypto")]
type Seed = [u8; 32];

#[cfg(feature = "full_crypto")]
fn derive_hard_junction(secret_seed: &Seed, cc: &[u8; 32]) -> Seed {
	("Secp256r1HDKD", secret_seed, cc).using_encoded(blake2_256)
}

#[cfg(feature = "full_crypto")]
pub enum DeriveError {
	SoftKeyInPath,
}

#[cfg(feature = "full_crypto")]
#[derive(Clone)]
pub struct Pair {
	public: Public,
	secret: SigningKey,
}

#[cfg(feature = "full_crypto")]
impl TraitPair for Pair {
	type Public = Public;
	type Seed = Seed;
	type Signature = Signature;
	type DeriveError = DeriveError;

	#[cfg(feature = "std")]
	fn generate_with_phrase(password: Option<&str>) -> (Pair, String, Seed) {
		let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
		let phrase = mnemonic.phrase();
		let (pair, seed) = Self::from_phrase(phrase, password)
			.expect("All phrases generated by Mnemonic are valid; qed");
		(pair, phrase.to_owned(), seed)
	}

	#[cfg(feature = "std")]
	fn from_phrase(
		phrase: &str,
		password: Option<&str>,
	) -> Result<(Pair, Seed), SecretStringError> {
		let big_seed = substrate_bip39::seed_from_entropy(
			Mnemonic::from_phrase(phrase, Language::English)
				.map_err(|_| SecretStringError::InvalidPhrase)?
				.entropy(),
			password.unwrap_or(""),
		)
		.map_err(|_| SecretStringError::InvalidSeed)?;
		let mut seed = Seed::default();
		seed.copy_from_slice(&big_seed[0..32]);
		Self::from_seed_slice(&big_seed[0..32]).map(|x| (x, seed))
	}

	fn from_seed(seed: &Seed) -> Pair {
		Self::from_seed_slice(&seed[..]).expect("seed has valid length; qed")
	}

	fn from_seed_slice(seed_slice: &[u8]) -> Result<Pair, SecretStringError> {
		let secret =
			SigningKey::from_bytes(seed_slice).map_err(|_| SecretStringError::InvalidSeed)?;
		let public = PublicKey::from(secret.verifying_key());
		let public = Public::from_slice(public.to_encoded_point(true).as_bytes()).unwrap();
		Ok(Pair { public, secret })
	}

	fn derive<Iter: Iterator<Item = DeriveJunction>>(
		&self,
		path: Iter,
		_seed: Option<Seed>,
	) -> Result<(Pair, Option<Seed>), DeriveError> {
		let mut acc = self.seed();
		for j in path {
			match j {
				DeriveJunction::Soft(_cc) => return Err(DeriveError::SoftKeyInPath),
				DeriveJunction::Hard(cc) => acc = derive_hard_junction(&acc, &cc),
			}
		}
		Ok((Self::from_seed(&acc), Some(acc)))
	}

	fn public(&self) -> Public {
		self.public
	}

	fn sign(&self, message: &[u8]) -> Signature {
		self.sign_prehashed(&sha2_256(message))
	}

	fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool {
		Self::verify_weak(&sig.0, &message, &pubkey.0)
	}

	fn verify_weak<P: AsRef<[u8]>, M: AsRef<[u8]>>(sig: &[u8], message: M, pubkey: P) -> bool {
		Self::verify_prehash_unchecked(sig, &sha2_256(message.as_ref()), pubkey.as_ref())
	}

	fn to_raw_vec(&self) -> Vec<u8> {
		self.seed().to_vec()
	}
}

#[cfg(feature = "full_crypto")]
impl Pair {
	pub fn seed(&self) -> Seed {
		Seed::from(self.secret.to_bytes())
	}

	pub fn sign_prehashed(&self, message: &[u8; 32]) -> Signature {
		let sig: EcdsaSignature = self.secret.sign_prehash(message).unwrap();
		Signature(sig.to_bytes().into())
	}

	pub fn verify_prehashed(sig: &Signature, message: &[u8; 32], pubkey: &Public) -> bool {
		Self::verify_prehash_unchecked(sig.as_ref(), message, pubkey.as_ref())
	}

	fn verify_prehash_unchecked(sig: &[u8], message: &[u8; 32], pubkey: &[u8]) -> bool {
		let ver = VerifyingKey::from_sec1_bytes(pubkey).unwrap();
		let sig = EcdsaSignature::try_from(sig).unwrap();
		ver.verify_prehash(message, &sig).is_ok()
	}
}

impl CryptoType for Public {
	#[cfg(feature = "full_crypto")]
	type Pair = Pair;
}

impl CryptoType for Signature {
	#[cfg(feature = "full_crypto")]
	type Pair = Pair;
}

#[cfg(feature = "full_crypto")]
impl CryptoType for Pair {
	type Pair = Pair;
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json;
	use sp_core::crypto::{
		default_ss58_version, set_default_ss58_version, PublicError, Ss58AddressFormat,
		Ss58AddressFormatRegistry, DEV_PHRASE,
	};

	#[test]
	fn default_phrase_should_be_used() {
		assert_eq!(
			Pair::from_string("//Alice///password", None).unwrap().public(),
			Pair::from_string(&format!("{}//Alice", DEV_PHRASE), Some("password"))
				.unwrap()
				.public(),
		);
	}

	#[test]
	fn seed_and_derive_should_work() {
		let seed = array_bytes::hex2array_unchecked(
			"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		);
		let pair = Pair::from_seed(&seed);
		assert_eq!(pair.seed(), seed);
		let path = vec![DeriveJunction::Hard([0u8; 32])];
		let derived = pair.derive(path.into_iter(), None).ok().unwrap();
		assert_eq!(
			derived.0.seed(),
			array_bytes::hex2array_unchecked::<32>(
				"7ef571a7bc8f2e0c4b641e30d55018a6058b6003506967150fcc4349c1af4cbb"
			)
		);
	}

	#[test]
	fn test_vector_should_work() {
		let pair = Pair::from_seed(&array_bytes::hex2array_unchecked(
			"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		));
		let public = pair.public();
		assert_eq!(
			public,
			Public::try_from(
				&array_bytes::hex2bytes_unchecked("667fef5f7578a801037ed144092dcf7c7c44e3bf3e09cfc8a67fcf70fcd8123a3a29739e598824b33aef8068c6057a2f9fa1661253f1ea799e6ef7ce89a00438")[..],
			).unwrap(),
		);
		let message = b"";
		let signature: [u8; 64] = array_bytes::hex2array_unchecked("bee4fbe2396943104fd1693487f6285f584a37b1f01660b52cf4bd4bd6137c808c8ee0e95af6e4f218a919021fb4fa975647bc773f4074cf8141c3963398e013");
		let signature = Signature::try_from(&signature[..]).unwrap();
		assert!(pair.sign(&message[..]) == signature);
		assert!(Pair::verify(&signature, &message[..], &public));
	}

	#[test]
	fn test_vector_by_string_should_work() {
		let pair = Pair::from_string(
			"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
			None,
		)
		.unwrap();
		let public = pair.public();
		assert_eq!(
			public,
			Public::try_from(
				&array_bytes::hex2bytes_unchecked("667fef5f7578a801037ed144092dcf7c7c44e3bf3e09cfc8a67fcf70fcd8123a3a29739e598824b33aef8068c6057a2f9fa1661253f1ea799e6ef7ce89a00438")[..],
			).unwrap(),
		);
		let message = b"";
		let signature: [u8; 64] = array_bytes::hex2array_unchecked("bee4fbe2396943104fd1693487f6285f584a37b1f01660b52cf4bd4bd6137c808c8ee0e95af6e4f218a919021fb4fa975647bc773f4074cf8141c3963398e013");
		let signature = Signature::try_from(&signature[..]).unwrap();
		assert!(pair.sign(&message[..]) == signature);
		assert!(Pair::verify(&signature, &message[..], &public));
	}

	#[test]
	fn generated_pair_should_work() {
		let (pair, _) = Pair::generate();
		let public = pair.public();
		let message = b"Something important";
		let signature = pair.sign(&message[..]);
		assert!(Pair::verify(&signature, &message[..], &public));
		assert!(!Pair::verify(&signature, b"Something else", &public));
	}

	#[test]
	fn seeded_pair_should_work() {
		let pair = Pair::from_seed(b"12345678901234567890123456789012");
		let public = pair.public();
		assert_eq!(
			public,
			Public::try_from(
				&array_bytes::hex2bytes_unchecked("6223e55c8ab75407c630ca15cc0281db060bcb47b99fd9d89239806c1088741b7763fc4f252598cd63a29d72507f9f1c161781b8a3174218e1f3c0edb419b831")[..],
			).unwrap(),
		);
		let message = array_bytes::hex2bytes_unchecked("2f8c6129d816cf51c374bc7f08c3e63ed156cf78aefb4a6550d97b87997977ee00000000000000000200d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a4500000000000000");
		let signature = pair.sign(&message[..]);
		println!("Correct signature: {:?}", signature);
		assert!(Pair::verify(&signature, &message[..], &public));
		assert!(!Pair::verify(&signature, "Other message", &public));
	}

	#[test]
	fn generate_with_phrase_recovery_possible() {
		let (pair1, phrase, _) = Pair::generate_with_phrase(None);
		let (pair2, _) = Pair::from_phrase(&phrase, None).unwrap();

		assert_eq!(pair1.public(), pair2.public());
	}

	#[test]
	fn generate_with_password_phrase_recovery_possible() {
		let (pair1, phrase, _) = Pair::generate_with_phrase(Some("password"));
		let (pair2, _) = Pair::from_phrase(&phrase, Some("password")).unwrap();

		assert_eq!(pair1.public(), pair2.public());
	}

	#[test]
	fn password_does_something() {
		let (pair1, phrase, _) = Pair::generate_with_phrase(Some("password"));
		let (pair2, _) = Pair::from_phrase(&phrase, None).unwrap();

		assert_ne!(pair1.public(), pair2.public());
	}

	#[test]
	fn ss58check_roundtrip_works() {
		let pair = Pair::from_seed(b"12345678901234567890123456789012");
		let public = pair.public();
		let s = public.to_ss58check();
		println!("Correct: {}", s);
		let cmp = Public::from_ss58check(&s).unwrap();
		assert_eq!(cmp, public);
	}

	#[test]
	fn ss58check_format_check_works() {
		let pair = Pair::from_seed(b"12345678901234567890123456789012");
		let public = pair.public();
		let format = Ss58AddressFormatRegistry::Reserved46Account.into();
		let s = public.to_ss58check_with_version(format);
		assert_eq!(Public::from_ss58check_with_version(&s), Err(PublicError::FormatNotAllowed));
	}

	#[test]
	fn ss58check_full_roundtrip_works() {
		let pair = Pair::from_seed(b"12345678901234567890123456789012");
		let public = pair.public();
		let format = Ss58AddressFormatRegistry::PolkadotAccount.into();
		let s = public.to_ss58check_with_version(format);
		let (k, f) = Public::from_ss58check_with_version(&s).unwrap();
		assert_eq!(k, public);
		assert_eq!(f, format);

		let format = Ss58AddressFormat::custom(64);
		let s = public.to_ss58check_with_version(format);
		let (k, f) = Public::from_ss58check_with_version(&s).unwrap();
		assert_eq!(k, public);
		assert_eq!(f, format);
	}

	#[test]
	fn ss58check_custom_format_works() {
		// We need to run this test in its own process to not interfere with other tests running in
		// parallel and also relying on the ss58 version.
		if std::env::var("RUN_CUSTOM_FORMAT_TEST") == Ok("1".into()) {
			// temp save default format version
			let default_format = default_ss58_version();
			// set current ss58 version is custom "200" `Ss58AddressFormat::Custom(200)`

			set_default_ss58_version(Ss58AddressFormat::custom(200));
			// custom addr encoded by version 200
			let addr = "4pbsSkWcBaYoFHrKJZp5fDVUKbqSYD9dhZZGvpp3vQ5ysVs5ybV";
			Public::from_ss58check(addr).unwrap();

			set_default_ss58_version(default_format);
			// set current ss58 version to default version
			let addr = "KWAfgC2aRG5UVD6CpbPQXCx4YZZUhvWqqAJE6qcYc9Rtr6g5C";
			Public::from_ss58check(addr).unwrap();

			println!("CUSTOM_FORMAT_SUCCESSFUL");
		} else {
			let executable = std::env::current_exe().unwrap();
			let output = std::process::Command::new(executable)
				.env("RUN_CUSTOM_FORMAT_TEST", "1")
				.args(&["--nocapture", "ss58check_custom_format_works"])
				.output()
				.unwrap();

			let output = String::from_utf8(output.stdout).unwrap();
			assert!(output.contains("CUSTOM_FORMAT_SUCCESSFUL"));
		}
	}

	#[test]
	fn signature_serialization_works() {
		let pair = Pair::from_seed(b"12345678901234567890123456789012");
		let message = b"Something important";
		let signature = pair.sign(&message[..]);
		let serialized_signature = serde_json::to_string(&signature).unwrap();
		// Signature is 64 bytes, so 128 chars + 2 quote chars
		assert_eq!(serialized_signature.len(), 130);
		let signature = serde_json::from_str(&serialized_signature).unwrap();
		assert!(Pair::verify(&signature, &message[..], &pair.public()));
	}

	#[test]
	fn signature_serialization_doesnt_panic() {
		fn deserialize_signature(text: &str) -> Result<Signature, serde_json::error::Error> {
			serde_json::from_str(text)
		}
		assert!(deserialize_signature("Not valid json.").is_err());
		assert!(deserialize_signature("\"Not an actual signature.\"").is_err());
		// Poorly-sized
		assert!(deserialize_signature("\"abc123\"").is_err());
	}

	#[test]
	fn sign_prehashed_works() {
		let (pair, _, _) = Pair::generate_with_phrase(Some("password"));

		// `msg` shouldn't be mangled
		let msg = [0u8; 32];
		let sig1 = pair.sign_prehashed(&msg);
		let sig2: Signature = {
			let sig: EcdsaSignature = pair.secret.sign_prehash(&msg).unwrap();
			Signature(sig.to_bytes().into())
		};
		assert_eq!(sig1, sig2);

		// signature is actually different
		let sig2 = pair.sign(&msg);
		assert_ne!(sig1, sig2);

		// using pre-hashed `msg` works
		let msg = b"this should be hashed";
		let sig1 = pair.sign_prehashed(&sha2_256(msg));
		let sig2 = pair.sign(msg);
		assert_eq!(sig1, sig2);
	}

	#[test]
	fn verify_prehashed_works() {
		let (pair, _, _) = Pair::generate_with_phrase(Some("password"));

		// `msg` and `sig` match
		let msg = sha2_256(b"this should be hashed");
		let sig = pair.sign_prehashed(&msg);
		assert!(Pair::verify_prehashed(&sig, &msg, &pair.public()));

		// `msg` and `sig` don't match
		let msg = sha2_256(b"this is a different message");
		assert!(!Pair::verify_prehashed(&sig, &msg, &pair.public()));
	}
}
