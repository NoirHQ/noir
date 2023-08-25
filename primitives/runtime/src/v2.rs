#[cfg(feature = "serde")]
pub use serde::{Deserialize, Serialize};

use derive_more::{From, TryInto};
use np_crypto::{p256, webauthn};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{crypto::AccountId32, ecdsa, ed25519, sr25519};
use sp_runtime::{
	traits::{IdentifyAccount, Lazy, Verify},
	RuntimeDebug,
};
use sp_std::prelude::*;

/// Signature verify that can work with any known signature types.
#[derive(Eq, PartialEq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UniversalSignature {
	/// A Ed25519 signature.
	Ed25519(ed25519::Signature),
	/// A Sr25519 signature.
	Sr25519(sr25519::Signature),
	/// A Secp256k1 signature.
	Secp256k1(ecdsa::Signature),
	/// A P-256 signature.
	P256(p256::Signature),
	/// A WebAuthn ES256 signature.
	WebAuthn(webauthn::Signature),
}

impl Verify for UniversalSignature {
	type Signer = UniversalSigner;

	fn verify<L: Lazy<[u8]>>(&self, mut msg: L, signer: &AccountId32) -> bool {
		match (self, signer) {
			(Self::Ed25519(ref sig), who) => match ed25519::Public::try_from(who.as_ref()) {
				Ok(signer) => sig.verify(msg, &signer),
				Err(()) => false,
			},
			(Self::Sr25519(ref sig), who) => match sr25519::Public::try_from(who.as_ref()) {
				Ok(signer) => sig.verify(msg, &signer),
				Err(()) => false,
			},
			(Self::Secp256k1(ref sig), who) => {
				let m = sp_io::hashing::blake2_256(msg.get());
				match sp_io::crypto::secp256k1_ecdsa_recover_compressed(sig.as_ref(), &m) {
					Ok(pubkey) =>
						&sp_io::hashing::blake2_256(pubkey.as_ref()) ==
							<dyn AsRef<[u8; 32]>>::as_ref(who),
					_ => false,
				}
			},
			(Self::P256(ref sig), who) => {
				let m = sp_io::hashing::blake2_256(msg.get());
				match np_io::crypto::p256_recover_compressed(sig.as_ref(), &m) {
					Ok(pubkey) =>
						&sp_io::hashing::blake2_256(pubkey.as_ref()) ==
							<dyn AsRef<[u8; 32]>>::as_ref(who),
					_ => false,
				}
			},
			_ => false,
			/*

			(Self::WebAuthn(ref sig), who) => match p256::Public::try_from(&who.0[2..]) {
				Ok(signer) => np_io::crypto::webauthn_verify(sig, msg.get(), &signer),
				Err(_) => false,
			},
			*/
		}
	}
}

/// Public key for any known crypto algorithm.
#[derive(
	Eq, PartialEq, Ord, PartialOrd, Clone, Encode, Decode, RuntimeDebug, TypeInfo, From, TryInto,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UniversalSigner {
	/// A Ed25519 identity.
	Ed25519(ed25519::Public),
	/// A Sr25519 identity.
	Sr25519(sr25519::Public),
	/// A Secp256k1 identity.
	Secp256k1(ecdsa::Public),
	/// A P-256 identity.
	P256(p256::Public),
}

impl AsRef<[u8]> for UniversalSigner {
	fn as_ref(&self) -> &[u8] {
		match *self {
			Self::Ed25519(ref who) => who.as_ref(),
			Self::Sr25519(ref who) => who.as_ref(),
			Self::Secp256k1(ref who) => who.as_ref(),
			Self::P256(ref who) => who.as_ref(),
		}
	}
}

impl IdentifyAccount for UniversalSigner {
	type AccountId = AccountId32;

	fn into_account(self) -> Self::AccountId {
		match self {
			Self::Ed25519(who) => <[u8; 32]>::from(who).into(),
			Self::Sr25519(who) => <[u8; 32]>::from(who).into(),
			Self::Secp256k1(who) => sp_io::hashing::blake2_256(who.as_ref()).into(),
			Self::P256(who) => sp_io::hashing::blake2_256(who.as_ref()).into(),
		}
	}
}
