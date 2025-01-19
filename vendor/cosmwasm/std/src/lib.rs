// #[cfg(not(feature = "std"))]
// core::compile_error!(
//     r#"Please enable `cosmwasm-std`'s `std` feature, as we might move existing functionality to
// that feature in the future. Builds without the std feature are currently not expected to work. If
// you need no_std support see #1484. "#
// );

#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate alloc;

extern crate const_hex as hex;

// Exposed on all platforms

mod __internal;
mod addresses;
mod assertions;
mod binary;
mod checksum;
mod coin;
mod coins;
mod conversion;
mod deps;
mod encoding;
mod errors;
mod forward_ref;
mod hex_binary;
mod ibc;
mod import_helpers;
#[cfg(feature = "iterator")]
mod iterator;
mod math;
mod metadata;
mod never;
mod pagination;
mod panic;
mod query;
mod results;
mod sections;
mod serde;
mod stdack;
mod storage;
mod timestamp;
mod traits;
mod types;

/// This module is to simplify no_std imports
pub(crate) mod prelude;

/// This modules is very advanced and will not be used directly by the vast majority of users.
/// We want to offer it to ensure a stable storage key composition system but don't encourage
/// contract devs to use it directly.
pub mod storage_keys;

#[cfg(feature = "iterator")]
pub use crate::iterator::{Order, Record};
#[cfg(all(feature = "stargate", feature = "cosmwasm_1_2"))]
pub use crate::results::WeightedVoteOption;
#[cfg(feature = "staking")]
pub use crate::results::{DistributionMsg, StakingMsg};
#[cfg(feature = "stargate")]
pub use crate::results::{GovMsg, VoteOption};
#[allow(deprecated)]
pub use crate::serde::{
	from_binary, from_json, from_slice, to_binary, to_json_binary, to_json_string, to_json_vec,
	to_vec,
};
pub use crate::{
	addresses::{instantiate2_address, Addr, CanonicalAddr, Instantiate2AddressError},
	binary::Binary,
	checksum::{Checksum, ChecksumError},
	coin::{coin, coins, has_coins, Coin},
	coins::Coins,
	deps::{Deps, DepsMut, OwnedDeps},
	encoding::{from_base64, from_hex, to_base64, to_hex},
	errors::{
		AggregationError, CheckedFromRatioError, CheckedMultiplyFractionError,
		CheckedMultiplyRatioError, CoinFromStrError, CoinsError, ConversionOverflowError,
		DivideByZeroError, DivisionError, OverflowError, OverflowOperation, PairingEqualityError,
		RecoverPubkeyError, RoundDownOverflowError, RoundUpOverflowError, StdError, StdResult,
		SystemError, VerificationError,
	},
	hex_binary::HexBinary,
	ibc::{
		Ibc3ChannelOpenResponse, IbcAckCallbackMsg, IbcAcknowledgement, IbcBasicResponse,
		IbcCallbackRequest, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg,
		IbcChannelOpenMsg, IbcChannelOpenResponse, IbcDestinationCallbackMsg, IbcDstCallback,
		IbcEndpoint, IbcMsg, IbcOrder, IbcPacket, IbcPacketAckMsg, IbcPacketReceiveMsg,
		IbcPacketTimeoutMsg, IbcReceiveResponse, IbcSourceCallbackMsg, IbcSrcCallback, IbcTimeout,
		IbcTimeoutBlock, IbcTimeoutCallbackMsg, TransferMsgBuilder,
	},
	math::{
		Decimal, Decimal256, Decimal256RangeExceeded, DecimalRangeExceeded, Fraction, Int128,
		Int256, Int512, Int64, Isqrt, SignedDecimal, SignedDecimal256,
		SignedDecimal256RangeExceeded, SignedDecimalRangeExceeded, Uint128, Uint256, Uint512,
		Uint64,
	},
	metadata::{DenomMetadata, DenomUnit},
	never::Never,
	pagination::PageRequest,
	query::{
		AllBalanceResponse, AllDelegationsResponse, AllDenomMetadataResponse,
		AllValidatorsResponse, BalanceResponse, BankQuery, BondedDenomResponse, ChannelResponse,
		CodeInfoResponse, ContractInfoResponse, CustomQuery, DecCoin, Delegation,
		DelegationResponse, DelegationRewardsResponse, DelegationTotalRewardsResponse,
		DelegatorReward, DelegatorValidatorsResponse, DelegatorWithdrawAddressResponse,
		DenomMetadataResponse, DistributionQuery, FullDelegation, GrpcQuery, IbcQuery,
		ListChannelsResponse, PortIdResponse, QueryRequest, StakingQuery, SupplyResponse,
		Validator, ValidatorResponse, WasmQuery,
	},
	results::{
		attr, wasm_execute, wasm_instantiate, AnyMsg, Attribute, BankMsg, ContractResult,
		CosmosMsg, CustomMsg, Empty, Event, MsgResponse, QueryResponse, Reply, ReplyOn, Response,
		SubMsg, SubMsgResponse, SubMsgResult, SystemResult, WasmMsg,
	},
	stdack::StdAck,
	storage::MemoryStorage,
	timestamp::Timestamp,
	traits::{Api, HashFunction, Querier, QuerierResult, QuerierWrapper, Storage},
	types::{BlockInfo, ContractInfo, Env, MessageInfo, TransactionInfo},
};

// Exposed in wasm build only

#[cfg(target_arch = "wasm32")]
mod exports;
#[cfg(target_arch = "wasm32")]
mod imports;
#[cfg(target_arch = "wasm32")]
mod memory; // Used by exports and imports only. This assumes pointers are 32 bit long, which makes it
			// untestable on dev machines.

#[cfg(target_arch = "wasm32")]
pub use crate::exports::{
	do_execute, do_ibc_destination_callback, do_ibc_source_callback, do_instantiate, do_migrate,
	do_query, do_reply, do_sudo,
};
#[cfg(all(feature = "stargate", target_arch = "wasm32"))]
pub use crate::exports::{
	do_ibc_channel_close, do_ibc_channel_connect, do_ibc_channel_open, do_ibc_packet_ack,
	do_ibc_packet_receive, do_ibc_packet_timeout,
};
#[cfg(target_arch = "wasm32")]
pub use crate::imports::{ExternalApi, ExternalQuerier, ExternalStorage};

/// Exposed for testing only
/// Both unit tests and integration tests are compiled to native code, so everything in here does
/// not need to compile to Wasm.
//#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "std")]
pub mod testing;

pub use cosmwasm_core::{BLS12_381_G1_GENERATOR, BLS12_381_G2_GENERATOR};
pub use cosmwasm_derive::entry_point;
