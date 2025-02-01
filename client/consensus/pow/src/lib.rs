// This file is part of Noir.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Noir Proof of work consensus.
//!
//! To use this engine, you can need to have a struct that implements
//! [`PowAlgorithm`]. After that, pass an instance of the struct, along
//! with other necessary client references to [`import_queue`] to setup
//! the queue.
//!
//! This library also comes with an async mining worker, which can be
//! started via the [`start_mining_worker`] function. It returns a worker
//! handle together with a future. The future must be pulled. Through
//! the worker handle, you can pull the metadata needed to start the
//! mining process via [`MiningHandle::metadata`], and then do the actual
//! mining on a standalone thread. Finally, when a seal is found, call
//! [`MiningHandle::submit`] to build the block.
//!
//! The auxiliary storage for PoW engine only stores the total difficulty.
//! For other storage requirements for particular PoW algorithm (such as
//! the actual difficulty for each particular blocks), you can take a client
//! reference in your [`PowAlgorithm`] implementation, and use a separate prefix
//! for the auxiliary storage. It is also possible to just use the runtime
//! as the storage, but it is not recommended as it won't work well with light
//! clients.

mod aux_schema;
mod digests;
mod worker;

pub use aux_schema::*;
pub use digests::*;
pub use worker::*;

use futures::{Future, StreamExt};
use log::*;
use nc_consensus::PreDigestProvider;
use np_consensus_pow::{Seal, POW_ENGINE_ID};
use parity_scale_codec::{Decode, Encode};
use sc_client_api::{self, backend::AuxStore, BlockOf, BlockchainEvents};
use sc_consensus::{
	BasicQueue, BlockCheckParams, BlockImport, BlockImportParams, BoxJustificationImport,
	ForkChoiceStrategy, ImportResult, Verifier,
};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{
	BlockOrigin, Environment, Error as ConsensusError, Proposer, SelectChain, SyncOracle,
};
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{
	generic::{BlockId, Digest, DigestItem},
	traits::{Block as BlockT, Header as HeaderT, Saturating},
};
use std::{cmp::Ordering, marker::PhantomData, sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;

const LOG_TARGET: &str = "pow";

#[derive(Debug, thiserror::Error)]
pub enum Error<B: BlockT> {
	#[error("Header uses the wrong engine: {}", String::from_utf8_lossy(.0))]
	WrongEngine([u8; 4]),
	#[error("Header {0:?} has a bad seal")]
	HeaderBadSeal(B::Hash),
	#[error("Header {0:?} is unsealed")]
	HeaderUnsealed(B::Hash),
	#[error("Preliminary verification failed")]
	PreliminaryVerificationFailed,
	#[error("Creating inherents failed: {0}")]
	CreateInherents(sp_inherents::Error),
	#[error("Checking inherents failed: {0}")]
	CheckInherents(sp_inherents::Error),
	#[error("Checking inherents unhandled error: {}", String::from_utf8_lossy(.0))]
	CheckInherentsUnhandled(sp_inherents::InherentIdentifier),
	#[error("Multiple pre-runtime digests")]
	MultiplePreRuntimeDigests,
	#[error(transparent)]
	Client(sp_blockchain::Error),
	#[error(transparent)]
	Codec(parity_scale_codec::Error),
	#[error("{0}")]
	Other(String),
}

impl<B: BlockT> From<Error<B>> for String {
	fn from(error: Error<B>) -> String {
		error.to_string()
	}
}

impl<B: BlockT> From<Error<B>> for ConsensusError {
	fn from(error: Error<B>) -> ConsensusError {
		ConsensusError::ClientImport(error.to_string())
	}
}

/// Intermediate value passed to block importer.
#[derive(Encode, Decode, Clone, Debug, Default)]
pub struct PowIntermediate<Difficulty> {
	/// Difficulty of the block, if known.
	pub difficulty: Option<Difficulty>,
}

/// Intermediate key for PoW engine.
pub static INTERMEDIATE_KEY: &[u8] = b"pow1";

/// Algorithm used for proof of work.
pub trait PowAlgorithm<B: BlockT> {
	/// Difficulty for the algorithm.
	type Difficulty: Saturating + Default + Encode + Decode + Ord + Clone + Copy;

	/// Get the next block's difficulty.
	///
	/// This function will be called twice during the import process, so the implementation
	/// should be properly cached.
	fn difficulty(&self, parent: B::Hash) -> Result<Self::Difficulty, Error<B>>;

	/// Verify that the seal is valid against given pre hash when parent block is not yet imported.
	///
	/// None means that preliminary verify is not available for this algorithm.
	fn preliminary_verify(
		&self,
		_pre_hash: &B::Hash,
		_seal: &Seal,
	) -> Result<Option<bool>, Error<B>> {
		Ok(None)
	}

	/// Break a fork choice tie.
	///
	/// By default this chooses the earliest block seen. Using uniform tie
	/// breaking algorithms will help to protect against selfish mining.
	///
	/// Returns if the new seal should be considered best block.
	fn break_tie(&self, _own_seal: &Seal, _new_seal: &Seal) -> bool {
		false
	}

	/// Verify that the difficulty is valid against given seal.
	///
	/// This function should return the actual difficulty of the block.
	/// None means that the seal is invalid.
	fn verify(
		&self,
		parent: &BlockId<B>,
		pre_hash: &B::Hash,
		pre_digest: Option<&[u8]>,
		seal: &Seal,
		difficulty: Self::Difficulty,
	) -> Result<Option<Self::Difficulty>, Error<B>>;

	/// Calculates the difficulty of the given header.
	fn calculate_difficulty(&self, _header: &B::Header) -> Result<Self::Difficulty, Error<B>> {
		unimplemented!()
	}
}

/// A block importer for PoW.
pub struct PowBlockImport<B, I, C, SC, A> {
	inner: I,
	client: Arc<C>,
	select_chain: SC,
	algorithm: A,
	_marker: PhantomData<B>,
}

impl<B: BlockT, I: Clone, C, SC: Clone, A: Clone> Clone for PowBlockImport<B, I, C, SC, A> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			client: self.client.clone(),
			select_chain: self.select_chain.clone(),
			algorithm: self.algorithm.clone(),
			_marker: Default::default(),
		}
	}
}

impl<B, I, C, SC, A> PowBlockImport<B, I, C, SC, A>
where
	B: BlockT,
	I: BlockImport<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf,
	C::Api: BlockBuilderApi<B>,
	A: PowAlgorithm<B>,
{
	/// Create a new block import suitable to be used in PoW
	pub fn new(inner: I, client: Arc<C>, select_chain: SC, algorithm: A) -> Self {
		Self { inner, client, select_chain, algorithm, _marker: Default::default() }
	}
}

#[async_trait::async_trait]
impl<B, I, C, SC, A> BlockImport<B> for PowBlockImport<B, I, C, SC, A>
where
	B: BlockT,
	I: BlockImport<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf,
	C::Api: BlockBuilderApi<B>,
	SC: SelectChain<B>,
	A: PowAlgorithm<B> + Send + Sync,
	A::Difficulty: Send + 'static,
{
	type Error = ConsensusError;

	async fn check_block(&self, block: BlockCheckParams<B>) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block).await.map_err(Into::into)
	}

	async fn import_block(
		&self,
		mut block: BlockImportParams<B>,
	) -> Result<ImportResult, Self::Error> {
		let best_header = self
			.select_chain
			.best_chain()
			.await
			.map_err(|e| format!("Fetch best chain failed via select chain: {}", e))
			.map_err(ConsensusError::ChainLookup)?;
		let best_hash = best_header.hash();
		let parent_hash = *block.header.parent_hash();

		let best_weight =
			aux_schema::load_block_weight(&*self.client, &best_hash).map_err(Error::Client::<B>)?;
		let mut block_weight = aux_schema::load_block_weight(&*self.client, &parent_hash)
			.map_err(Error::Client::<B>)?;

		let inner_seal = fetch_seal::<B>(block.post_digests.last(), block.header.hash())?;

		let intermediate =
			block.remove_intermediate::<PowIntermediate<A::Difficulty>>(INTERMEDIATE_KEY)?;

		let target_difficulty = match intermediate.difficulty {
			Some(difficulty) => difficulty,
			None => self.algorithm.difficulty(parent_hash)?,
		};

		let pre_hash = block.header.hash();
		let pre_digest = find_pre_digest::<B>(&block.header)?;
		let difficulty = match self.algorithm.verify(
			&BlockId::hash(parent_hash),
			&pre_hash,
			pre_digest.as_ref().map(|v| &v[..]),
			&inner_seal,
			target_difficulty,
		)? {
			Some(difficulty) => difficulty,
			None => return Err(Error::<B>::HeaderBadSeal(block.post_hash()).into()),
		};

		block_weight += difficulty;

		aux_schema::write_block_weight(block.post_hash(), block_weight, |insert| {
			block
				.auxiliary
				.extend(insert.iter().map(|(k, v)| (k.to_vec(), Some(v.to_vec()))));
		});
		if block.fork_choice.is_none() {
			block.fork_choice =
				Some(ForkChoiceStrategy::Custom(match block_weight.cmp(&best_weight) {
					Ordering::Less => false,
					Ordering::Greater => true,
					Ordering::Equal =>
						if block.header.number() < best_header.number() {
							true
						} else {
							match block.origin {
								BlockOrigin::Own => true,
								_ => {
									let best_inner_seal = fetch_seal::<B>(
										best_header.digest().logs.last(),
										best_hash,
									)?;

									self.algorithm.break_tie(&best_inner_seal, &inner_seal)
								},
							}
						},
				}));
		}

		self.inner.import_block(block).await.map_err(Into::into)
	}
}

/// A verifier for PoW blocks.
pub struct PowVerifier<B: BlockT, C, A, CIDP> {
	client: Arc<C>,
	algorithm: A,
	create_inherent_data_providers: CIDP,
	_marker: PhantomData<B>,
}

impl<B, C, A, CIDP> PowVerifier<B, C, A, CIDP>
where
	B: BlockT,
	C: ProvideRuntimeApi<B>,
	C::Api: BlockBuilderApi<B>,
	A: PowAlgorithm<B>,
	CIDP: CreateInherentDataProviders<B, ()>,
{
	pub fn new(client: Arc<C>, algorithm: A, create_inherent_data_providers: CIDP) -> Self {
		Self { client, algorithm, create_inherent_data_providers, _marker: Default::default() }
	}

	fn check_header(&self, mut header: B::Header) -> Result<(B::Header, DigestItem), Error<B>> {
		let hash = header.hash();

		let (seal, inner_seal) = match header.digest_mut().pop() {
			Some(DigestItem::Seal(id, seal)) =>
				if id == POW_ENGINE_ID {
					(DigestItem::Seal(id, seal.clone()), seal)
				} else {
					return Err(Error::WrongEngine(id))
				},
			_ => return Err(Error::HeaderUnsealed(hash)),
		};

		let pre_hash = header.hash();

		if !self.algorithm.preliminary_verify(&pre_hash, &inner_seal)?.unwrap_or(true) {
			return Err(Error::PreliminaryVerificationFailed)
		}

		Ok((header, seal))
	}

	async fn check_inherents(
		&self,
		block: B,
		at_hash: B::Hash,
		create_inherent_data_providers: CIDP::InherentDataProviders,
	) -> Result<(), Error<B>> {
		let inherent_data = create_inherent_data_providers
			.create_inherent_data()
			.await
			.map_err(Error::<B>::CreateInherents)?;

		let inherent_res = self
			.client
			.runtime_api()
			.check_inherents(at_hash, block, inherent_data)
			.map_err(|e| Error::Client(e.into()))?;

		if !inherent_res.ok() {
			for (identifier, error) in inherent_res.into_errors() {
				match create_inherent_data_providers.try_handle_error(&identifier, &error).await {
					Some(res) => res.map_err(Error::CheckInherents)?,
					None => return Err(Error::CheckInherentsUnhandled(identifier)),
				}
			}
		}

		Ok(())
	}
}

#[async_trait::async_trait]
impl<B, C, A, CIDP> Verifier<B> for PowVerifier<B, C, A, CIDP>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + Send + Sync,
	C::Api: BlockBuilderApi<B>,
	A: PowAlgorithm<B> + Send + Sync,
	A::Difficulty: 'static + Send,
	CIDP: CreateInherentDataProviders<B, ()> + Send + Sync,
{
	async fn verify(
		&self,
		mut block: BlockImportParams<B>,
	) -> Result<BlockImportParams<B>, String> {
		let hash = block.header.hash();
		let parent_hash = *block.header.parent_hash();

		let create_inherent_data_providers = self
			.create_inherent_data_providers
			.create_inherent_data_providers(parent_hash, ())
			.await
			.map_err(|e| Error::<B>::Client(ConsensusError::from(e).into()))?;

		let (checked_header, seal) = self.check_header(block.header)?;

		if let Some(inner_body) = block.body.take() {
			let check_block = B::new(checked_header.clone(), inner_body);

			if !block.state_action.skip_execution_checks() {
				self.check_inherents(
					check_block.clone(),
					parent_hash,
					create_inherent_data_providers,
				)
				.await?;
			}

			block.body = Some(check_block.deconstruct().1);
		}

		let intermediate = PowIntermediate::<A::Difficulty> { difficulty: None };
		block.header = checked_header;
		block.post_digests.push(seal);
		block.insert_intermediate(INTERMEDIATE_KEY, intermediate);
		block.post_hash = Some(hash);

		Ok(block)
	}
}

/// The PoW import queue type.
pub type PowImportQueue<B> = BasicQueue<B>;

/// Parameters passed to [`import_queue`].
pub struct ImportQueueParams<'a, B, I, C, A, CIDP, S> {
	/// The block import to use.
	pub block_import: I,
	/// The justification import.
	pub justification_import: Option<BoxJustificationImport<B>>,
	/// The client to interact with the chain.
	pub client: Arc<C>,
	/// PoW algorithm.
	pub algorithm: A,
	/// Something that can create the inherent data providers.
	pub create_inherent_data_providers: CIDP,
	/// The spawner to spawn background tasks.
	pub spawner: &'a S,
	/// The prometheus registry.
	pub registry: Option<&'a Registry>,
}

/// Import queue for PoW engine.
pub fn import_queue<B, I, C, A, CIDP, S>(
	ImportQueueParams {
		block_import,
		justification_import,
		client,
		algorithm,
		create_inherent_data_providers,
		spawner,
		registry,
	}: ImportQueueParams<B, I, C, A, CIDP, S>,
) -> Result<PowImportQueue<B>, sp_consensus::Error>
where
	B: BlockT,
	I: BlockImport<B, Error = ConsensusError> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: BlockBuilderApi<B>,
	A: PowAlgorithm<B> + Clone + Send + Sync + 'static,
	A::Difficulty: Send,
	CIDP: CreateInherentDataProviders<B, ()> + Send + 'static,
	S: sp_core::traits::SpawnEssentialNamed,
{
	let verifier = PowVerifier::new(client, algorithm, create_inherent_data_providers);

	Ok(BasicQueue::new(verifier, Box::new(block_import), justification_import, spawner, registry))
}

/// Parameters used to start a mining worker.
pub struct PowParams<C, SC, I, A, E, SO, L, CIDP, PP> {
	/// The client to interact with the chain.
	pub client: Arc<C>,
	/// A select chain implementation to select the best block.
	pub select_chain: SC,
	/// The block import.
	pub block_import: I,
	/// PoW algorithm.
	pub algorithm: A,
	/// The proposer factory to build proposer instances.
	pub proposer_factory: E,
	/// The sync oracle that can give us the current sync status.
	pub sync_oracle: SO,
	/// Hook into the sync module to control the justification sync process.
	pub justification_sync_link: L,
	/// Something that can create the inherent data providers.
	pub create_inherent_data_providers: CIDP,
	/// Pre-runtime digest to be inserted into blocks.
	pub pre_digest_provider: PP,
	/// Timeout for importing a block.
	pub timeout: Duration,
	/// Maximum time allowed for building a block.
	pub build_time: Duration,
}

/// Start the pow worker. This function provides the necessary helper functions that can
/// be used to implement a miner. However, it does not do the CPU-intensive mining itself.
#[allow(clippy::type_complexity)]
pub fn start_pow<Block, C, S, I, Algorithm, E, SO, L, CIDP, PP>(
	PowParams {
		client,
		select_chain,
		block_import,
		algorithm,
		mut proposer_factory,
		sync_oracle,
		justification_sync_link,
		create_inherent_data_providers,
		pre_digest_provider,
		timeout,
		build_time,
	}: PowParams<C, S, I, Algorithm, E, SO, L, CIDP, PP>,
) -> (
	PowWorker<Block, Algorithm, L, <E::Proposer as Proposer<Block>>::Proof, I>,
	impl Future<Output = ()>,
)
where
	Block: BlockT,
	C: BlockchainEvents<Block> + 'static,
	S: SelectChain<Block> + 'static,
	I: BlockImport<Block> + Send + Sync + 'static,
	Algorithm: PowAlgorithm<Block> + Clone,
	Algorithm::Difficulty: Send + 'static,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
	SO: SyncOracle + Clone + Send + Sync + 'static,
	L: sc_consensus::JustificationSyncLink<Block>,
	CIDP: CreateInherentDataProviders<Block, ()>,
	PP: PreDigestProvider,
{
	let mut timer = UntilImportedOrTimeout::new(client.import_notification_stream(), timeout);
	let worker = PowWorker::new(algorithm.clone(), block_import, justification_sync_link);
	let worker_ret = worker.clone();

	let task = async move {
		loop {
			if timer.next().await.is_none() {
				break
			}

			if sync_oracle.is_major_syncing() {
				debug!(target: LOG_TARGET, "Skipping proposal due to sync.");
				worker.on_major_syncing();
				continue
			}

			let best_header = match select_chain.best_chain().await {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to pull new block for authoring. \
						 Select best chain error: {}",
						err
					);
					continue
				},
			};
			let best_hash = best_header.hash();

			// The worker is locked for the duration of the whole proposing period. Within this
			// period, the mining target is outdated and useless anyway.

			let difficulty = match algorithm.difficulty(best_hash) {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to propose new block for authoring. \
						 Fetch difficulty failed: {}",
						err,
					);
					continue
				},
			};

			let inherent_data_providers = match create_inherent_data_providers
				.create_inherent_data_providers(best_hash, ())
				.await
			{
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to propose new block for authoring. \
						 Creating inherent data providers failed: {}",
						err,
					);
					continue
				},
			};

			let inherent_data = match inherent_data_providers.create_inherent_data().await {
				Ok(r) => r,
				Err(e) => {
					warn!(
						target: LOG_TARGET,
						"Unable to propose new block for authoring. \
						 Creating inherent data failed: {}",
						e,
					);
					continue
				},
			};

			let mut inherent_digest = Digest::default();
			let mut pre_digest = None;

			match pre_digest_provider.pre_digest(best_hash.as_ref()).await {
				Ok(items) =>
					for item in items {
						if let DigestItem::PreRuntime(POW_ENGINE_ID, ref data) = item {
							pre_digest = Some(data.clone());
						}
						inherent_digest.push(item);
					},
				Err(e) => {
					warn!(
						target: LOG_TARGET,
						"Invalid pre-runtime digest: {}",
						e,
					);
					continue
				},
			}

			let proposer = match proposer_factory.init(&best_header).await {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to propose new block for authoring. \
						 Creating proposer failed: {:?}",
						err,
					);
					continue
				},
			};

			let proposal =
				match proposer.propose(inherent_data, inherent_digest, build_time, None).await {
					Ok(x) => x,
					Err(err) => {
						warn!(
							target: LOG_TARGET,
							"Unable to propose new block for authoring. \
							 Creating proposal failed: {}",
							err,
						);
						continue
					},
				};

			let build = MiningBuild::<Block, Algorithm, _> {
				metadata: MiningMetadata {
					best_hash,
					pre_hash: proposal.block.header().hash(),
					pre_digest,
					difficulty,
				},
				proposal,
			};

			worker.on_build(build);
		}
	};

	(worker_ret, task)
}

/// Find PoW pre-runtime.
fn find_pre_digest<B: BlockT>(header: &B::Header) -> Result<Option<Vec<u8>>, Error<B>> {
	let mut pre_digest: Option<_> = None;
	for log in header.digest().logs() {
		trace!(target: LOG_TARGET, "Checking log {:?}, looking for pre runtime digest", log);
		match (log, pre_digest.is_some()) {
			(DigestItem::PreRuntime(POW_ENGINE_ID, _), true) =>
				return Err(Error::MultiplePreRuntimeDigests),
			(DigestItem::PreRuntime(POW_ENGINE_ID, v), false) => {
				pre_digest = Some(v.clone());
			},
			(_, _) => trace!(target: LOG_TARGET, "Ignoring digest not meant for us"),
		}
	}

	Ok(pre_digest)
}

/// Fetch PoW seal.
fn fetch_seal<B: BlockT>(digest: Option<&DigestItem>, hash: B::Hash) -> Result<Vec<u8>, Error<B>> {
	match digest {
		Some(DigestItem::Seal(id, seal)) =>
			if id == &POW_ENGINE_ID {
				Ok(seal.clone())
			} else {
				Err(Error::<B>::WrongEngine(*id))
			},
		_ => Err(Error::<B>::HeaderUnsealed(hash)),
	}
}
