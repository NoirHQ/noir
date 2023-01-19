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

//! Simple ECDSA secp256r1 API.

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime_interface::pass_by::PassByInner;

#[cfg(feature = "std")]
use bip39::{Language, Mnemonic, MnemonicType};
#[cfg(feature = "full_crypto")]
use p256::{
	ecdsa::{
		signature::hazmat::{PrehashSigner, PrehashVerifier},
		Signature as EcdsaSignature, SigningKey, VerifyingKey,
	},
	elliptic_curve::sec1::ToEncodedPoint,
	PublicKey,
};
#[cfg(feature = "std")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "std")]
use sp_core::crypto::Ss58Codec;
use sp_core::crypto::{
	ByteArray, CryptoType, CryptoTypeId, CryptoTypePublicPair, Derive, Public as TraitPublic,
	UncheckedFrom,
};
#[cfg(feature = "full_crypto")]
use sp_core::{
	crypto::{DeriveJunction, Pair as TraitPair, SecretStringError},
	hashing::blake2_256,
};
#[cfg(feature = "full_crypto")]
use sp_std::vec::Vec;

/// An identifier used to match public keys against ecdsa P-256 keys
pub const CRYPTO_ID: CryptoTypeId = CryptoTypeId(*b"p256");

/// A secret seed (which is bytewise essentially equivalent to a SecretKey).
///
/// We need it as a different type because `Seed` is expected to be AsRef<[u8]>.
#[cfg(feature = "full_crypto")]
type Seed = [u8; 32];

/// The ECDSA P-256 compressed public key.
#[cfg_attr(feature = "full_crypto", derive(Hash))]
#[derive(
	Clone,
	Copy,
	Encode,
	Decode,
	PassByInner,
	MaxEncodedLen,
	TypeInfo,
	Eq,
	PartialEq,
	PartialOrd,
	Ord,
)]
pub struct Public(pub [u8; 33]);

impl Public {
	/// A new instance from the given 33-byte `data`.
	///
	/// NOTE: No checking goes on to ensure this is a real public key. Only use it if
	/// you are certain that the array actually is a pubkey. GIGO!
	pub fn from_raw(data: [u8; 33]) -> Self {
		Self(data)
	}

	/// Create a new instance from the given full public key.
	///
	/// This will convert the full public key into the compressed format.
	#[cfg(feature = "std")]
	pub fn from_full(full: &[u8]) -> Result<Self, ()> {
		let pubkey = if full.len() == 64 {
			// Tag it as uncompressed public key.
			let mut tagged_full = [0u8; 65];
			tagged_full[0] = 0x04;
			tagged_full[1..].copy_from_slice(full);
			PublicKey::from_sec1_bytes(&tagged_full)
		} else {
			PublicKey::from_sec1_bytes(full)
		};
		match pubkey {
			Ok(k) => Self::try_from(k.to_encoded_point(true).to_bytes().as_ref()),
			Err(..) => Err(()),
		}
	}
}

impl ByteArray for Public {
	const LEN: usize = 33;
}

impl TraitPublic for Public {
	fn to_public_crypto_pair(&self) -> CryptoTypePublicPair {
		CryptoTypePublicPair(CRYPTO_ID, self.to_raw_vec())
	}
}

impl From<Public> for CryptoTypePublicPair {
	fn from(key: Public) -> Self {
		(&key).into()
	}
}

impl From<&Public> for CryptoTypePublicPair {
	fn from(key: &Public) -> Self {
		CryptoTypePublicPair(CRYPTO_ID, key.to_raw_vec())
	}
}

impl Derive for Public {}

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

	fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
		if data.len() != Self::LEN {
			return Err(())
		}
		let mut r = [0u8; Self::LEN];
		r.copy_from_slice(data);
		Ok(Self::unchecked_from(r))
	}
}

#[cfg(feature = "full_crypto")]
impl From<Pair> for Public {
	fn from(x: Pair) -> Self {
		x.public()
	}
}

impl UncheckedFrom<[u8; 33]> for Public {
	fn unchecked_from(x: [u8; 33]) -> Self {
		Public(x)
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

/// A signature (a 512-bit value).
#[cfg_attr(feature = "full_crypto", derive(Hash))]
#[derive(Encode, Decode, MaxEncodedLen, PassByInner, TypeInfo, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

impl TryFrom<&[u8]> for Signature {
	type Error = ();

	fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
		if data.len() == 64 {
			let mut inner = [0u8; 64];
			inner.copy_from_slice(data);
			Ok(Signature(inner))
		} else {
			Err(())
		}
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

impl Clone for Signature {
	fn clone(&self) -> Self {
		let mut r = [0u8; 64];
		r.copy_from_slice(&self.0[..]);
		Signature(r)
	}
}

impl Default for Signature {
	fn default() -> Self {
		Signature([0u8; 64])
	}
}

impl From<Signature> for [u8; 64] {
	fn from(v: Signature) -> [u8; 64] {
		v.0
	}
}

impl AsRef<[u8; 64]> for Signature {
	fn as_ref(&self) -> &[u8; 64] {
		&self.0
	}
}

impl AsRef<[u8]> for Signature {
	fn as_ref(&self) -> &[u8] {
		&self.0[..]
	}
}

impl AsMut<[u8]> for Signature {
	fn as_mut(&mut self) -> &mut [u8] {
		&mut self.0[..]
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

impl UncheckedFrom<[u8; 64]> for Signature {
	fn unchecked_from(data: [u8; 64]) -> Signature {
		Signature(data)
	}
}

impl Signature {
	/// A new instance from the given 64-byte `data`.
	///
	/// NOTE: No checking goes on to ensure this is a real signature. Only use it if
	/// you are certain that the array actually is a signature. GIGO!
	pub fn from_raw(data: [u8; 64]) -> Signature {
		Signature(data)
	}

	/// A new instance from the given slice that should be 64 bytes long.
	///
	/// NOTE: No checking goes on to ensure this is a real signature. Only use it if
	/// you are certain that the array actually is a signature. GIGO!
	pub fn from_slice(data: &[u8]) -> Option<Self> {
		if data.len() != 64 {
			return None
		}
		let mut r = [0u8; 64];
		r.copy_from_slice(data);
		Some(Signature(r))
	}
}

/// Derive a single hard junction.
#[cfg(feature = "full_crypto")]
fn derive_hard_junction(secret_seed: &Seed, cc: &[u8; 32]) -> Seed {
	("Secp256r1HDKD", secret_seed, cc).using_encoded(sp_core::hashing::blake2_256)
}

/// An error when deriving a key.
#[cfg(feature = "full_crypto")]
pub enum DeriveError {
	/// A soft key was found in the path (and is unsupported).
	SoftKeyInPath,
}

/// A key pair.
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

	/// Generate new secure (random) key pair and provide the recovery phrase.
	///
	/// You can recover the same key later with `from_phrase`.
	#[cfg(feature = "std")]
	fn generate_with_phrase(password: Option<&str>) -> (Pair, String, Seed) {
		let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
		let phrase = mnemonic.phrase();
		let (pair, seed) = Self::from_phrase(phrase, password)
			.expect("All phrases generated by Mnemonic are valid; qed");
		(pair, phrase.to_owned(), seed)
	}

	/// Generate key pair from given recovery phrase and password.
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

	/// Make a new key pair from secret seed material.
	///
	/// You should never need to use this; generate(), generate_with_phrase
	fn from_seed(seed: &Seed) -> Pair {
		Self::from_seed_slice(&seed[..]).expect("seed has valid length; qed")
	}

	/// Make a new key pair from secret seed material. The slice must be 32 bytes long or it
	/// will return `None`.
	///
	/// You should never need to use this; generate(), generate_with_phrase
	fn from_seed_slice(seed_slice: &[u8]) -> Result<Pair, SecretStringError> {
		let secret =
			SigningKey::from_bytes(seed_slice).map_err(|_| SecretStringError::InvalidSeed)?;
		let public = PublicKey::from(secret.verifying_key());
		let public = Public::from_slice(public.to_encoded_point(true).as_bytes())
			.expect("public key has valid length; qed");
		Ok(Pair { public, secret })
	}

	/// Derive a child key from a series of given junctions.
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

	/// Get the public key.
	fn public(&self) -> Public {
		self.public
	}

	/// Sign a message.
	fn sign(&self, message: &[u8]) -> Signature {
		self.sign_prehashed(&blake2_256(message))
	}

	/// Verify a signature on a message. Returns true if the signature is good.
	fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool {
		Self::verify_weak(&sig.0, &message, &pubkey.0)
	}

	/// Verify a signature on a message. Returns true if the signature is good.
	///
	/// This doesn't use the type system to ensure that `sig` and `pubkey` are the correct
	/// size. Use it only if you're coming from byte buffers and need the speed.
	fn verify_weak<P: AsRef<[u8]>, M: AsRef<[u8]>>(sig: &[u8], message: M, pubkey: P) -> bool {
		self::verify_prehashed(sig, &blake2_256(message.as_ref()), pubkey.as_ref())
	}

	/// Return a vec filled with raw data.
	fn to_raw_vec(&self) -> Vec<u8> {
		self.seed().to_vec()
	}
}

fn verify_prehashed(sig: &[u8], message: &[u8; 32], pubkey: &[u8]) -> bool {
	let pubkey = VerifyingKey::from_sec1_bytes(pubkey).expect("");
	let sig = EcdsaSignature::try_from(sig).expect("");
	pubkey.verify_prehash(message, &sig).is_ok()
}

#[cfg(feature = "full_crypto")]
impl Pair {
	/// Get the seed for this key.
	pub fn seed(&self) -> Seed {
		Seed::from(self.secret.to_bytes())
	}

	/// Sign a pre-hashed message
	pub fn sign_prehashed(&self, message: &[u8; 32]) -> Signature {
		let sig: EcdsaSignature = self.secret.sign_prehash(message).expect("");
		Signature(sig.to_bytes().into())
	}

	/// Verify a signature on a pre-hashed message. Return `true` if the signature is valid
	/// and thus matches the given `public` key.
	pub fn verify_prehashed(sig: &Signature, message: &[u8; 32], pubkey: &Public) -> bool {
		self::verify_prehashed(sig.as_ref(), message, pubkey.as_ref())
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
