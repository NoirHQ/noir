#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use codec::Codec;

// Cosmwasm Runtime API declaration.
sp_api::decl_runtime_apis! {
	pub trait CosmwasmRuntimeApi<Error>
	where
		Error: Codec
	{
		fn query(
			contract: String,
			gas: u64,
			query_request: Vec<u8>,
		) -> Result<Vec<u8>, Error>;
	}
}
