// This file is part of Substrate.

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

//! Utilities for generating and verifying BEEFY warp sync proofs.

use codec::{Decode, DecodeAll, Encode};

use crate::{
	find_authorities_change,
	justification::{verify_with_validator_set, BeefyVersionedFinalityProof},
};
use sc_client_api::Backend as ClientBackend;
use sc_network_sync::strategy::warp::{
	EncodedProof, VerificationResult, Verifier, WarpSyncProvider,
};
use sp_blockchain::{Backend as BlockchainBackend, HeaderBackend};
use sp_consensus_beefy::{AuthorityIdBound, ValidatorSet, BEEFY_ENGINE_ID};
use sp_runtime::{
	traits::{Block as BlockT, Header as HeaderT, One},
	Justifications,
};

use std::{marker::PhantomData, sync::Arc};

/// The maximum size in bytes of the BEEFY `WarpSyncProof`.
pub(super) const MAX_WARP_SYNC_PROOF_SIZE: usize = 8 * 1024 * 1024;

/// A proof of a single BEEFY validator set transition.
///
/// Each fragment corresponds to a mandatory block (a session-boundary block that contains
/// `ConsensusLog::AuthoritiesChange` in its header digest). The `justification` is a
/// SCALE-encoded `VersionedFinalityProof` proving that the mandatory block was finalized by
/// ≥2/3+1 of the *previous* BEEFY validator set.
#[derive(Decode, Encode, Debug)]
pub struct WarpSyncFragment<Block: BlockT> {
	/// Mandatory block header (contains `ConsensusLog::AuthoritiesChange` digest).
	pub header: Block::Header,
	/// SCALE-encoded `VersionedFinalityProof` proving finality of this block by the previous
	/// validator set.
	pub justification: Vec<u8>,
}

/// An accumulated proof of multiple BEEFY validator set transitions.
#[derive(Decode, Encode)]
pub struct WarpSyncProof<Block: BlockT> {
	pub(crate) fragments: Vec<WarpSyncFragment<Block>>,
	/// Whether this proof covers all available mandatory blocks (i.e. BEEFY has finalized them
	/// all up to the current tip). A verifier receiving a proof with `is_finished = true` may
	/// consider warp sync complete.
	pub(crate) is_finished: bool,
}

impl<Block: BlockT> WarpSyncProof<Block> {
	/// Generate a BEEFY warp sync proof starting at `begin`.
	///
	/// Scans finalized block headers from `begin` to the current finalized tip, collecting
	/// mandatory blocks (those with `AuthoritiesChange` in their digest) that have BEEFY
	/// justifications, up to [`MAX_WARP_SYNC_PROOF_SIZE`].
	///
	/// Returns `is_finished = true` if all available mandatory blocks were collected without
	/// hitting the size limit.
	fn generate<Backend, AuthorityId>(
		backend: &Backend,
		begin: Block::Hash,
	) -> Result<WarpSyncProof<Block>, Error>
	where
		Backend: ClientBackend<Block>,
		AuthorityId: AuthorityIdBound,
	{
		let blockchain = backend.blockchain();

		let begin_number = blockchain
			.number(begin)?
			.ok_or_else(|| Error::InvalidRequest("Missing start block".to_string()))?;

		let finalized_number = blockchain.info().finalized_number;

		if begin_number > finalized_number {
			return Err(Error::InvalidRequest("Start block is not finalized".to_string()));
		}

		let canon_hash = blockchain.hash(begin_number)?.expect(
			"begin number is lower than or equal to finalized number; \
			 all blocks up to finalized number must have been imported; qed.",
		);

		if canon_hash != begin {
			return Err(Error::InvalidRequest(
				"Start block is not in the finalized chain".to_string(),
			));
		}

		let mut fragments = Vec::new();
		let mut proof_encoded_len = 0;
		let mut proof_limit_reached = false;

		// Scan forward from begin_number + 1 (begin itself was already processed by the verifier).
		let mut current_number = begin_number + One::one();
		while current_number <= finalized_number {
			let hash = match blockchain.hash(current_number)? {
				Some(h) => h,
				None => break,
			};

			let header = match blockchain.header(hash)? {
				Some(h) => h,
				None => break,
			};

			// Mandatory blocks have an AuthoritiesChange digest entry.
			if find_authorities_change::<Block, AuthorityId>(&header).is_some() {
				let justification = blockchain
					.justifications(hash)?
					.and_then(|j| j.into_justification(BEEFY_ENGINE_ID));

				match justification {
					Some(justification_bytes) => {
						let fragment = WarpSyncFragment {
							header: header.clone(),
							justification: justification_bytes,
						};
						let fragment_size = fragment.encoded_size();

						// Reserve 50 bytes for the Vec length prefix and the `is_finished` bool.
						if proof_encoded_len + fragment_size >= MAX_WARP_SYNC_PROOF_SIZE - 50 {
							proof_limit_reached = true;
							break;
						}

						proof_encoded_len += fragment_size;
						fragments.push(fragment);
					},
					None => {
						// Mandatory block without a BEEFY justification: BEEFY has not yet
						// finalized this block. Stop here; there are no more valid fragments.
						break;
					},
				}
			}

			current_number = current_number + One::one();
		}

		let is_finished = !proof_limit_reached;
		Ok(WarpSyncProof { fragments, is_finished })
	}
}

/// Warp proof processing error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Decoding error.
	#[error("Failed to decode: {0}.")]
	DecodeScale(#[from] codec::Error),
	/// Client backend error.
	#[error("{0}")]
	Client(#[from] sp_blockchain::Error),
	/// Invalid request data.
	#[error("{0}")]
	InvalidRequest(String),
	/// Invalid warp proof.
	#[error("{0}")]
	InvalidProof(String),
	/// Missing header or justification data.
	#[error("Missing required data to be able to answer request.")]
	MissingData,
}

/// Implements the network API for BEEFY warp sync.
pub struct NetworkProvider<Block: BlockT, Backend: ClientBackend<Block>, AuthorityId: AuthorityIdBound>
{
	backend: Arc<Backend>,
	/// The genesis BEEFY validator set, used to seed the verifier.
	genesis_validator_set: ValidatorSet<AuthorityId>,
	_phantom: PhantomData<Block>,
}

impl<Block, Backend, AuthorityId> NetworkProvider<Block, Backend, AuthorityId>
where
	Block: BlockT,
	Backend: ClientBackend<Block>,
	AuthorityId: AuthorityIdBound,
{
	/// Create a new [`NetworkProvider`].
	///
	/// `genesis_validator_set` must be the BEEFY validator set that was active at genesis (set id
	/// 0). The verifier uses it as the root of trust when validating warp sync proofs.
	pub fn new(
		backend: Arc<Backend>,
		genesis_validator_set: ValidatorSet<AuthorityId>,
	) -> Self {
		NetworkProvider { backend, genesis_validator_set, _phantom: PhantomData }
	}
}

/// Verifier state for BEEFY warp sync.
struct BeefyVerifier<Block: BlockT, AuthorityId: AuthorityIdBound> {
	/// The BEEFY validator set that is expected to have signed the next proof fragment.
	current_validator_set: ValidatorSet<AuthorityId>,
	/// Hash of the last verified block; the next proof request will start from here.
	next_proof_context: Block::Hash,
	/// Number of validator set transitions verified so far (for progress reporting).
	eras_synced: u64,
}

impl<Block, AuthorityId> Verifier<Block> for BeefyVerifier<Block, AuthorityId>
where
	Block: BlockT,
	AuthorityId: AuthorityIdBound,
{
	fn verify(
		&mut self,
		proof: &EncodedProof,
	) -> Result<VerificationResult<Block>, Box<dyn std::error::Error + Send + Sync>> {
		let EncodedProof(proof_bytes) = proof;
		let proof = WarpSyncProof::<Block>::decode_all(&mut proof_bytes.as_slice())
			.map_err(|e| format!("BEEFY warp proof decoding error: {:?}", e))?;

		if proof.fragments.is_empty() {
			return Err("Empty BEEFY warp proof".to_string().into());
		}

		let last_header = proof.fragments.last().map(|f| f.header.clone()).unwrap();
		let mut blocks = Vec::new();

		for fragment in &proof.fragments {
			let target_number = *fragment.header.number();

			// Decode the BEEFY finality proof.
			let finality_proof =
				BeefyVersionedFinalityProof::<Block, AuthorityId>::decode_all(
					&mut &fragment.justification[..],
				)
				.map_err(|e| {
					format!("Failed to decode BEEFY finality proof: {:?}", e)
				})?;

			// Verify the commitment against the current (pre-change) validator set.
			// This also checks that commitment.block_number == target_number and
			// commitment.validator_set_id == current_validator_set.id().
			verify_with_validator_set::<Block, AuthorityId>(
				target_number,
				&self.current_validator_set,
				&finality_proof,
			)
			.map_err(|(e, _)| {
				format!("BEEFY justification verification failed: {:?}", e)
			})?;

			// The fragment header must contain an AuthoritiesChange digest that gives us the new
			// validator set. Every mandatory block must have this.
			let new_validator_set =
				find_authorities_change::<Block, AuthorityId>(&fragment.header)
					.ok_or_else(|| {
						"BEEFY mandatory block is missing AuthoritiesChange digest".to_string()
					})?;

			// Advance to the new validator set.
			self.current_validator_set = new_validator_set;
			self.eras_synced += 1;

			let justifications = Justifications::new(vec![(
				BEEFY_ENGINE_ID,
				fragment.justification.clone(),
			)]);
			blocks.push((fragment.header.clone(), justifications));
		}

		self.next_proof_context = last_header.hash();

		if proof.is_finished {
			Ok(VerificationResult::Complete(last_header, blocks))
		} else {
			Ok(VerificationResult::Partial(blocks))
		}
	}

	fn next_proof_context(&self) -> Block::Hash {
		self.next_proof_context
	}

	fn status(&self) -> Option<String> {
		Some(format!("{} BEEFY sessions synced", self.eras_synced))
	}
}

impl<Block, Backend, AuthorityId> WarpSyncProvider<Block>
	for NetworkProvider<Block, Backend, AuthorityId>
where
	Block: BlockT,
	Backend: ClientBackend<Block>,
	AuthorityId: AuthorityIdBound,
{
	fn generate(
		&self,
		start: Block::Hash,
	) -> Result<EncodedProof, Box<dyn std::error::Error + Send + Sync>> {
		let proof =
			WarpSyncProof::<Block>::generate::<Backend, AuthorityId>(&*self.backend, start)
				.map_err(Box::new)?;
		Ok(EncodedProof(proof.encode()))
	}

	fn create_verifier(&self) -> Box<dyn Verifier<Block>> {
		let genesis_hash = self.backend.blockchain().info().genesis_hash;
		Box::new(BeefyVerifier {
			current_validator_set: self.genesis_validator_set.clone(),
			next_proof_context: genesis_hash,
			eras_synced: 0,
		})
	}
}
