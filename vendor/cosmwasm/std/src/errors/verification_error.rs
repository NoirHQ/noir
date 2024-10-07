use core::fmt::Debug;

use super::BT;

#[cfg(not(target_arch = "wasm32"))]
use cosmwasm_crypto::CryptoError;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum AggregationError {
	#[error("List of points is empty")]
	Empty,
	#[error("List is not an expected multiple")]
	NotMultiple,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum PairingEqualityError {
	#[error("List is not a multiple of 48")]
	NotMultipleG1,
	#[error("List is not a multiple of 96")]
	NotMultipleG2,
	#[error("Not the same amount of points passed")]
	UnequalPointAmount,
}

#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
	#[error("Aggregation error: {source}")]
	Aggregation { source: AggregationError },
	#[error("Batch error")]
	BatchErr,
	#[error("Generic error")]
	GenericErr,
	#[error("Invalid hash format")]
	InvalidHashFormat,
	#[error("Invalid signature format")]
	InvalidSignatureFormat,
	#[error("Invalid public key format")]
	InvalidPubkeyFormat,
	#[error("Invalid recovery parameter. Supported values: 0 and 1.")]
	InvalidRecoveryParam,
	#[error("Invalid point")]
	InvalidPoint,
	#[error("Unknown hash function")]
	UnknownHashFunction,
	#[error("Aggregation pairing equality error: {source}")]
	PairingEquality { source: PairingEqualityError },
	#[error("Unknown error: {error_code}")]
	UnknownErr { error_code: u32, backtrace: BT },
}

impl VerificationError {
	pub fn unknown_err(error_code: u32) -> Self {
		VerificationError::UnknownErr { error_code, backtrace: BT::capture() }
	}
}

impl PartialEq<VerificationError> for VerificationError {
	fn eq(&self, rhs: &VerificationError) -> bool {
		match self {
			VerificationError::Aggregation { source: lhs_source } => {
				matches!(rhs, VerificationError::Aggregation { source: rhs_source } if rhs_source == lhs_source)
			},
			VerificationError::PairingEquality { source: lhs_source } => {
				matches!(rhs, VerificationError::PairingEquality { source: rhs_source } if rhs_source == lhs_source)
			},
			VerificationError::BatchErr => matches!(rhs, VerificationError::BatchErr),
			VerificationError::GenericErr => matches!(rhs, VerificationError::GenericErr),
			VerificationError::InvalidHashFormat => {
				matches!(rhs, VerificationError::InvalidHashFormat)
			},
			VerificationError::InvalidPubkeyFormat => {
				matches!(rhs, VerificationError::InvalidPubkeyFormat)
			},
			VerificationError::InvalidSignatureFormat => {
				matches!(rhs, VerificationError::InvalidSignatureFormat)
			},
			VerificationError::InvalidRecoveryParam => {
				matches!(rhs, VerificationError::InvalidRecoveryParam)
			},
			VerificationError::InvalidPoint => matches!(rhs, VerificationError::InvalidPoint),
			VerificationError::UnknownHashFunction => {
				matches!(rhs, VerificationError::UnknownHashFunction)
			},
			VerificationError::UnknownErr { error_code, .. } =>
				if let VerificationError::UnknownErr { error_code: rhs_error_code, .. } = rhs {
					error_code == rhs_error_code
				} else {
					false
				},
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl From<CryptoError> for VerificationError {
	fn from(original: CryptoError) -> Self {
		match original {
			CryptoError::Aggregation {
				source: cosmwasm_crypto::AggregationError::Empty, ..
			} => VerificationError::Aggregation { source: AggregationError::Empty },
			CryptoError::Aggregation {
				source: cosmwasm_crypto::AggregationError::NotMultiple { .. },
				..
			} => VerificationError::Aggregation { source: AggregationError::NotMultiple },
			CryptoError::PairingEquality {
				source: cosmwasm_crypto::PairingEqualityError::NotMultipleG1 { .. },
				..
			} => VerificationError::PairingEquality { source: PairingEqualityError::NotMultipleG1 },
			CryptoError::PairingEquality {
				source: cosmwasm_crypto::PairingEqualityError::NotMultipleG2 { .. },
				..
			} => VerificationError::PairingEquality { source: PairingEqualityError::NotMultipleG2 },
			CryptoError::PairingEquality {
				source: cosmwasm_crypto::PairingEqualityError::UnequalPointAmount { .. },
				..
			} => VerificationError::PairingEquality {
				source: PairingEqualityError::UnequalPointAmount,
			},
			CryptoError::InvalidHashFormat { .. } => VerificationError::InvalidHashFormat,
			CryptoError::InvalidPubkeyFormat { .. } => VerificationError::InvalidPubkeyFormat,
			CryptoError::InvalidSignatureFormat { .. } => VerificationError::InvalidSignatureFormat,
			CryptoError::GenericErr { .. } => VerificationError::GenericErr,
			CryptoError::InvalidRecoveryParam { .. } => VerificationError::InvalidRecoveryParam,
			CryptoError::InvalidPoint { .. } => VerificationError::InvalidPoint,
			CryptoError::BatchErr { .. } => VerificationError::BatchErr,
			CryptoError::UnknownHashFunction { .. } => VerificationError::UnknownHashFunction,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// constructors
	#[test]
	fn unknown_err_works() {
		let error = VerificationError::unknown_err(123);
		match error {
			VerificationError::UnknownErr { error_code, .. } => assert_eq!(error_code, 123),
			_ => panic!("wrong error type!"),
		}
	}
}
