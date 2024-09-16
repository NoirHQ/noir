use codec::Codec;
pub use cosmwasm_api::CosmwasmApiServer;
use cosmwasm_runtime_api::CosmwasmRuntimeApi;
use jsonrpsee::{
	core::RpcResult,
	proc_macros::rpc,
	types::{ErrorObject, ErrorObjectOwned},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::traits::Block as BlockT;
use std::{marker::PhantomData, sync::Arc};

mod cosmwasm_api {
	use super::*;
	#[rpc(client, server)]
	pub trait CosmwasmApi<BlockHash, Error> {
		#[method(name = "cosmwasm_query")]
		fn query(
			&self,
			contract: String,
			gas: u64,
			query_request: Bytes,
			at: Option<BlockHash>,
		) -> RpcResult<Bytes>;
	}
}

pub struct Cosmwasm<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, M> Cosmwasm<C, M> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

fn runtime_error_into_rpc_error<E: ToString>(e: E) -> ErrorObjectOwned {
	ErrorObject::owned(
		9876, // no real reason for this value
		e.to_string(),
		None::<()>,
	)
}

impl<C, Block, Error> CosmwasmApiServer<<Block as BlockT>::Hash, Error>
	for Cosmwasm<C, (Block, Error)>
where
	Block: BlockT,
	Error: Send + Sync + 'static + Codec + AsRef<[u8]>,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: CosmwasmRuntimeApi<Block, Error>,
{
	fn query(
		&self,
		contract: String,
		gas: u64,
		query_request: Bytes,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Bytes> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		let runtime_api_result = api
			.query(at, contract, gas, query_request.to_vec())
			.map_err(runtime_error_into_rpc_error)?;
		runtime_api_result
			.map(Bytes::from)
			.map_err(|e| runtime_error_into_rpc_error(String::from_utf8_lossy(e.as_ref())))
	}
}
