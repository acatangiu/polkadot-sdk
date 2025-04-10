// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! The provisioner is responsible for assembling a relay chain block
//! from a set of available parachain candidates of its choice.

#![deny(missing_docs, unused_crate_dependencies)]

use bitvec::vec::BitVec;
use futures::{
	channel::oneshot, future::BoxFuture, prelude::*, stream::FuturesUnordered, FutureExt,
};
use futures_timer::Delay;

use polkadot_node_subsystem::{
	messages::{
		Ancestors, CandidateBackingMessage, ProspectiveParachainsMessage, ProvisionableData,
		ProvisionerInherentData, ProvisionerMessage,
	},
	overseer, ActivatedLeaf, ActiveLeavesUpdate, FromOrchestra, OverseerSignal, SpawnedSubsystem,
	SubsystemError,
};
use polkadot_node_subsystem_util::{request_availability_cores, TimeoutExt};
use polkadot_primitives::{
	vstaging::{BackedCandidate, CoreState},
	CandidateHash, CoreIndex, Hash, Id as ParaId, SignedAvailabilityBitfield, ValidatorIndex,
};
use std::collections::{BTreeMap, HashMap};

mod disputes;
mod error;
mod metrics;

pub use self::metrics::*;
use error::{Error, FatalResult};

#[cfg(test)]
mod tests;

/// How long to wait before proposing.
const PRE_PROPOSE_TIMEOUT: std::time::Duration = core::time::Duration::from_millis(2000);
/// Some timeout to ensure task won't hang around in the background forever on issues.
const SEND_INHERENT_DATA_TIMEOUT: std::time::Duration = core::time::Duration::from_millis(500);

const LOG_TARGET: &str = "parachain::provisioner";

/// The provisioner subsystem.
pub struct ProvisionerSubsystem {
	metrics: Metrics,
}

impl ProvisionerSubsystem {
	/// Create a new instance of the `ProvisionerSubsystem`.
	pub fn new(metrics: Metrics) -> Self {
		Self { metrics }
	}
}

/// A per-relay-parent state for the provisioning subsystem.
pub struct PerRelayParent {
	leaf: ActivatedLeaf,
	signed_bitfields: Vec<SignedAvailabilityBitfield>,
	is_inherent_ready: bool,
	awaiting_inherent: Vec<oneshot::Sender<ProvisionerInherentData>>,
}

impl PerRelayParent {
	fn new(leaf: ActivatedLeaf) -> Self {
		Self {
			leaf,
			signed_bitfields: Vec::new(),
			is_inherent_ready: false,
			awaiting_inherent: Vec::new(),
		}
	}
}

type InherentDelays = FuturesUnordered<BoxFuture<'static, Hash>>;

#[overseer::subsystem(Provisioner, error=SubsystemError, prefix=self::overseer)]
impl<Context> ProvisionerSubsystem {
	fn start(self, ctx: Context) -> SpawnedSubsystem {
		let future = async move {
			run(ctx, self.metrics)
				.await
				.map_err(|e| SubsystemError::with_origin("provisioner", e))
		}
		.boxed();

		SpawnedSubsystem { name: "provisioner-subsystem", future }
	}
}

#[overseer::contextbounds(Provisioner, prefix = self::overseer)]
async fn run<Context>(mut ctx: Context, metrics: Metrics) -> FatalResult<()> {
	let mut inherent_delays = InherentDelays::new();
	let mut per_relay_parent = HashMap::new();

	loop {
		let result =
			run_iteration(&mut ctx, &mut per_relay_parent, &mut inherent_delays, &metrics).await;

		match result {
			Ok(()) => break,
			err => crate::error::log_error(err)?,
		}
	}

	Ok(())
}

#[overseer::contextbounds(Provisioner, prefix = self::overseer)]
async fn run_iteration<Context>(
	ctx: &mut Context,
	per_relay_parent: &mut HashMap<Hash, PerRelayParent>,
	inherent_delays: &mut InherentDelays,
	metrics: &Metrics,
) -> Result<(), Error> {
	loop {
		futures::select! {
			from_overseer = ctx.recv().fuse() => {
				// Map the error to ensure that the subsystem exits when the overseer is gone.
				match from_overseer.map_err(Error::OverseerExited)? {
					FromOrchestra::Signal(OverseerSignal::ActiveLeaves(update)) =>
						handle_active_leaves_update(update, per_relay_parent, inherent_delays).await?,
					FromOrchestra::Signal(OverseerSignal::BlockFinalized(..)) => {},
					FromOrchestra::Signal(OverseerSignal::Conclude) => return Ok(()),
					FromOrchestra::Communication { msg } => {
						handle_communication(ctx, per_relay_parent, msg, metrics).await?;
					},
				}
			},
			hash = inherent_delays.select_next_some() => {
				if let Some(state) = per_relay_parent.get_mut(&hash) {
					state.is_inherent_ready = true;

					gum::trace!(
						target: LOG_TARGET,
						relay_parent = ?hash,
						"Inherent Data became ready"
					);

					let return_senders = std::mem::take(&mut state.awaiting_inherent);
					if !return_senders.is_empty() {
						send_inherent_data_bg(ctx, &state, return_senders, metrics.clone()).await?;
					}
				}
			}
		}
	}
}

async fn handle_active_leaves_update(
	update: ActiveLeavesUpdate,
	per_relay_parent: &mut HashMap<Hash, PerRelayParent>,
	inherent_delays: &mut InherentDelays,
) -> Result<(), Error> {
	gum::trace!(target: LOG_TARGET, "Handle ActiveLeavesUpdate");
	for deactivated in &update.deactivated {
		per_relay_parent.remove(deactivated);
	}

	if let Some(leaf) = update.activated {
		gum::trace!(target: LOG_TARGET, leaf_hash=?leaf.hash, "Adding delay");
		let delay_fut = Delay::new(PRE_PROPOSE_TIMEOUT).map(move |_| leaf.hash).boxed();
		per_relay_parent.insert(leaf.hash, PerRelayParent::new(leaf));
		inherent_delays.push(delay_fut);
	}

	Ok(())
}

#[overseer::contextbounds(Provisioner, prefix = self::overseer)]
async fn handle_communication<Context>(
	ctx: &mut Context,
	per_relay_parent: &mut HashMap<Hash, PerRelayParent>,
	message: ProvisionerMessage,
	metrics: &Metrics,
) -> Result<(), Error> {
	match message {
		ProvisionerMessage::RequestInherentData(relay_parent, return_sender) => {
			gum::trace!(target: LOG_TARGET, ?relay_parent, "Inherent data got requested.");

			if let Some(state) = per_relay_parent.get_mut(&relay_parent) {
				if state.is_inherent_ready {
					gum::trace!(target: LOG_TARGET, ?relay_parent, "Calling send_inherent_data.");
					send_inherent_data_bg(ctx, &state, vec![return_sender], metrics.clone())
						.await?;
				} else {
					gum::trace!(
						target: LOG_TARGET,
						?relay_parent,
						"Queuing inherent data request (inherent data not yet ready)."
					);
					state.awaiting_inherent.push(return_sender);
				}
			}
		},
		ProvisionerMessage::ProvisionableData(relay_parent, data) => {
			if let Some(state) = per_relay_parent.get_mut(&relay_parent) {
				let _timer = metrics.time_provisionable_data();

				gum::trace!(target: LOG_TARGET, ?relay_parent, "Received provisionable data: {:?}", &data);

				note_provisionable_data(state, data);
			}
		},
	}

	Ok(())
}

#[overseer::contextbounds(Provisioner, prefix = self::overseer)]
async fn send_inherent_data_bg<Context>(
	ctx: &mut Context,
	per_relay_parent: &PerRelayParent,
	return_senders: Vec<oneshot::Sender<ProvisionerInherentData>>,
	metrics: Metrics,
) -> Result<(), Error> {
	let leaf = per_relay_parent.leaf.clone();
	let signed_bitfields = per_relay_parent.signed_bitfields.clone();
	let mut sender = ctx.sender().clone();

	let bg = async move {
		let _timer = metrics.time_request_inherent_data();

		gum::trace!(
			target: LOG_TARGET,
			relay_parent = ?leaf.hash,
			"Sending inherent data in background."
		);

		let send_result =
			send_inherent_data(&leaf, &signed_bitfields, return_senders, &mut sender, &metrics) // Make sure call is not taking forever:
				.timeout(SEND_INHERENT_DATA_TIMEOUT)
				.map(|v| match v {
					Some(r) => r,
					None => Err(Error::SendInherentDataTimeout),
				});

		match send_result.await {
			Err(err) => {
				if let Error::CanceledBackedCandidates(_) = err {
					gum::debug!(
						target: LOG_TARGET,
						err = ?err,
						"Failed to assemble or send inherent data - block got likely obsoleted already."
					);
				} else {
					gum::warn!(target: LOG_TARGET, err = ?err, "failed to assemble or send inherent data");
				}
				metrics.on_inherent_data_request(Err(()));
			},
			Ok(()) => {
				metrics.on_inherent_data_request(Ok(()));
				gum::debug!(
					target: LOG_TARGET,
					signed_bitfield_count = signed_bitfields.len(),
					leaf_hash = ?leaf.hash,
					"inherent data sent successfully"
				);
				metrics.observe_inherent_data_bitfields_count(signed_bitfields.len());
			},
		}
	};

	ctx.spawn("send-inherent-data", bg.boxed())
		.map_err(|_| Error::FailedToSpawnBackgroundTask)?;

	Ok(())
}

fn note_provisionable_data(
	per_relay_parent: &mut PerRelayParent,
	provisionable_data: ProvisionableData,
) {
	match provisionable_data {
		ProvisionableData::Bitfield(_, signed_bitfield) =>
			per_relay_parent.signed_bitfields.push(signed_bitfield),
		// We choose not to punish these forms of misbehavior for the time being.
		// Risks from misbehavior are sufficiently mitigated at the protocol level
		// via reputation changes. Punitive actions here may become desirable
		// enough to dedicate time to in the future.
		ProvisionableData::MisbehaviorReport(_, _, _) => {},
		// We wait and do nothing here, preferring to initiate a dispute after the
		// parablock candidate is included for the following reasons:
		//
		// 1. A dispute for a candidate triggered at any point before the candidate
		// has been made available, including the backing stage, can't be
		// guaranteed to conclude. Non-concluding disputes are unacceptable.
		// 2. Candidates which haven't been made available don't pose a security
		// risk as they can not be included, approved, or finalized.
		//
		// Currently we rely on approval checkers to trigger disputes for bad
		// parablocks once they are included. But we can do slightly better by
		// allowing disagreeing backers to record their disagreement and initiate a
		// dispute once the parablock in question has been included. This potential
		// change is tracked by: https://github.com/paritytech/polkadot/issues/3232
		ProvisionableData::Dispute(_, _) => {},
	}
}

type CoreAvailability = BitVec<u8, bitvec::order::Lsb0>;

/// The provisioner is the subsystem best suited to choosing which specific
/// backed candidates and availability bitfields should be assembled into the
/// block. To engage this functionality, a
/// `ProvisionerMessage::RequestInherentData` is sent; the response is a set of
/// non-conflicting candidates and the appropriate bitfields. Non-conflicting
/// means that there are never two distinct parachain candidates included for
/// the same parachain and that new parachain candidates cannot be included
/// until the previous one either gets declared available or expired.
///
/// The main complication here is going to be around handling
/// occupied-core-assumptions. We might have candidates that are only
/// includable when some bitfields are included. And we might have candidates
/// that are not includable when certain bitfields are included.
///
/// When we're choosing bitfields to include, the rule should be simple:
/// maximize availability. So basically, include all bitfields. And then
/// choose a coherent set of candidates along with that.
async fn send_inherent_data(
	leaf: &ActivatedLeaf,
	bitfields: &[SignedAvailabilityBitfield],
	return_senders: Vec<oneshot::Sender<ProvisionerInherentData>>,
	from_job: &mut impl overseer::ProvisionerSenderTrait,
	metrics: &Metrics,
) -> Result<(), Error> {
	gum::trace!(
		target: LOG_TARGET,
		relay_parent = ?leaf.hash,
		"Requesting availability cores"
	);
	let availability_cores = request_availability_cores(leaf.hash, from_job)
		.await
		.await
		.map_err(|err| Error::CanceledAvailabilityCores(err))??;

	gum::trace!(
		target: LOG_TARGET,
		relay_parent = ?leaf.hash,
		"Selecting disputes"
	);

	let disputes = disputes::prioritized_selection::select_disputes(from_job, metrics, leaf).await;

	gum::trace!(
		target: LOG_TARGET,
		relay_parent = ?leaf.hash,
		"Selected disputes"
	);

	let bitfields = select_availability_bitfields(&availability_cores, bitfields, &leaf.hash);

	gum::trace!(
		target: LOG_TARGET,
		relay_parent = ?leaf.hash,
		"Selected bitfields"
	);

	let candidates = select_candidates(&availability_cores, &bitfields, leaf, from_job).await?;

	gum::trace!(
		target: LOG_TARGET,
		relay_parent = ?leaf.hash,
		"Selected candidates"
	);

	gum::debug!(
		target: LOG_TARGET,
		availability_cores_len = availability_cores.len(),
		disputes_count = disputes.len(),
		bitfields_count = bitfields.len(),
		candidates_count = candidates.len(),
		leaf_hash = ?leaf.hash,
		"inherent data prepared",
	);

	let inherent_data =
		ProvisionerInherentData { bitfields, backed_candidates: candidates, disputes };

	gum::trace!(
		target: LOG_TARGET,
		relay_parent = ?leaf.hash,
		"Sending back inherent data to requesters."
	);

	for return_sender in return_senders {
		return_sender
			.send(inherent_data.clone())
			.map_err(|_data| Error::InherentDataReturnChannel)?;
	}

	Ok(())
}

/// In general, we want to pick all the bitfields. However, we have the following constraints:
///
/// - not more than one per validator
/// - each 1 bit must correspond to an occupied core
///
/// If we have too many, an arbitrary selection policy is fine. For purposes of maximizing
/// availability, we pick the one with the greatest number of 1 bits.
///
/// Note: This does not enforce any sorting precondition on the output; the ordering there will be
/// unrelated to the sorting of the input.
fn select_availability_bitfields(
	cores: &[CoreState],
	bitfields: &[SignedAvailabilityBitfield],
	leaf_hash: &Hash,
) -> Vec<SignedAvailabilityBitfield> {
	let mut selected: BTreeMap<ValidatorIndex, SignedAvailabilityBitfield> = BTreeMap::new();

	gum::debug!(
		target: LOG_TARGET,
		bitfields_count = bitfields.len(),
		?leaf_hash,
		"bitfields count before selection"
	);

	'a: for bitfield in bitfields.iter().cloned() {
		if bitfield.payload().0.len() != cores.len() {
			gum::debug!(target: LOG_TARGET, ?leaf_hash, "dropping bitfield due to length mismatch");
			continue
		}

		let is_better = selected
			.get(&bitfield.validator_index())
			.map_or(true, |b| b.payload().0.count_ones() < bitfield.payload().0.count_ones());

		if !is_better {
			gum::trace!(
				target: LOG_TARGET,
				val_idx = bitfield.validator_index().0,
				?leaf_hash,
				"dropping bitfield due to duplication - the better one is kept"
			);
			continue
		}

		for (idx, _) in cores.iter().enumerate().filter(|v| !v.1.is_occupied()) {
			// Bit is set for an unoccupied core - invalid
			if *bitfield.payload().0.get(idx).as_deref().unwrap_or(&false) {
				gum::debug!(
					target: LOG_TARGET,
					val_idx = bitfield.validator_index().0,
					?leaf_hash,
					"dropping invalid bitfield - bit is set for an unoccupied core"
				);
				continue 'a
			}
		}

		let _ = selected.insert(bitfield.validator_index(), bitfield);
	}

	gum::debug!(
		target: LOG_TARGET,
		?leaf_hash,
		"selected {} of all {} bitfields (each bitfield is from a unique validator)",
		selected.len(),
		bitfields.len()
	);

	selected.into_values().collect()
}

/// Requests backable candidates from Prospective Parachains subsystem
/// based on core states.
async fn request_backable_candidates(
	availability_cores: &[CoreState],
	bitfields: &[SignedAvailabilityBitfield],
	relay_parent: &ActivatedLeaf,
	sender: &mut impl overseer::ProvisionerSenderTrait,
) -> Result<HashMap<ParaId, Vec<(CandidateHash, Hash)>>, Error> {
	let block_number_under_construction = relay_parent.number + 1;

	// Record how many cores are scheduled for each paraid. Use a BTreeMap because
	// we'll need to iterate through them.
	let mut scheduled_cores_per_para: BTreeMap<ParaId, usize> = BTreeMap::new();
	// The on-chain ancestors of a para present in availability-cores.
	let mut ancestors: HashMap<ParaId, Ancestors> =
		HashMap::with_capacity(availability_cores.len());

	for (core_idx, core) in availability_cores.iter().enumerate() {
		let core_idx = CoreIndex(core_idx as u32);
		match core {
			CoreState::Scheduled(scheduled_core) => {
				*scheduled_cores_per_para.entry(scheduled_core.para_id).or_insert(0) += 1;
			},
			CoreState::Occupied(occupied_core) => {
				let is_available = bitfields_indicate_availability(
					core_idx.0 as usize,
					bitfields,
					&occupied_core.availability,
				);

				if is_available {
					ancestors
						.entry(occupied_core.para_id())
						.or_default()
						.insert(occupied_core.candidate_hash);

					if let Some(ref scheduled_core) = occupied_core.next_up_on_available {
						// Request a new backable candidate for the newly scheduled para id.
						*scheduled_cores_per_para.entry(scheduled_core.para_id).or_insert(0) += 1;
					}
				} else if occupied_core.time_out_at <= block_number_under_construction {
					// Timed out before being available.

					if let Some(ref scheduled_core) = occupied_core.next_up_on_time_out {
						// Candidate's availability timed out, practically same as scheduled.
						*scheduled_cores_per_para.entry(scheduled_core.para_id).or_insert(0) += 1;
					}
				} else {
					// Not timed out and not available.
					ancestors
						.entry(occupied_core.para_id())
						.or_default()
						.insert(occupied_core.candidate_hash);
				}
			},
			CoreState::Free => continue,
		};
	}

	let mut selected_candidates: HashMap<ParaId, Vec<(CandidateHash, Hash)>> =
		HashMap::with_capacity(scheduled_cores_per_para.len());

	for (para_id, core_count) in scheduled_cores_per_para {
		let para_ancestors = ancestors.remove(&para_id).unwrap_or_default();

		let response = get_backable_candidates(
			relay_parent.hash,
			para_id,
			para_ancestors,
			core_count as u32,
			sender,
		)
		.await?;

		if response.is_empty() {
			gum::debug!(
				target: LOG_TARGET,
				leaf_hash = ?relay_parent.hash,
				?para_id,
				"No backable candidate returned by prospective parachains",
			);
			continue
		}

		selected_candidates.insert(para_id, response);
	}

	Ok(selected_candidates)
}

/// Determine which cores are free, and then to the degree possible, pick a candidate appropriate to
/// each free core.
async fn select_candidates(
	availability_cores: &[CoreState],
	bitfields: &[SignedAvailabilityBitfield],
	leaf: &ActivatedLeaf,
	sender: &mut impl overseer::ProvisionerSenderTrait,
) -> Result<Vec<BackedCandidate>, Error> {
	let relay_parent = leaf.hash;
	gum::trace!(
		target: LOG_TARGET,
		leaf_hash=?relay_parent,
		"before GetBackedCandidates"
	);

	let selected_candidates =
		request_backable_candidates(availability_cores, bitfields, leaf, sender).await?;
	gum::debug!(target: LOG_TARGET, ?selected_candidates, "Got backable candidates");

	// now get the backed candidates corresponding to these candidate receipts
	let (tx, rx) = oneshot::channel();
	sender.send_unbounded_message(CandidateBackingMessage::GetBackableCandidates(
		selected_candidates.clone(),
		tx,
	));
	let candidates = rx.await.map_err(|err| Error::CanceledBackedCandidates(err))?;
	gum::trace!(
		target: LOG_TARGET,
		leaf_hash=?relay_parent,
		"Got {} backed candidates", candidates.len()
	);

	// keep only one candidate with validation code.
	let mut with_validation_code = false;
	// merge the candidates into a common collection, preserving the order
	let mut merged_candidates = Vec::with_capacity(availability_cores.len());

	for para_candidates in candidates.into_values() {
		for candidate in para_candidates {
			if candidate.candidate().commitments.new_validation_code.is_some() {
				if with_validation_code {
					break
				} else {
					with_validation_code = true;
				}
			}

			merged_candidates.push(candidate);
		}
	}

	gum::debug!(
		target: LOG_TARGET,
		n_candidates = merged_candidates.len(),
		n_cores = availability_cores.len(),
		?relay_parent,
		"Selected backed candidates",
	);

	Ok(merged_candidates)
}

/// Requests backable candidates from Prospective Parachains based on
/// the given ancestors in the fragment chain. The ancestors may not be ordered.
async fn get_backable_candidates(
	relay_parent: Hash,
	para_id: ParaId,
	ancestors: Ancestors,
	count: u32,
	sender: &mut impl overseer::ProvisionerSenderTrait,
) -> Result<Vec<(CandidateHash, Hash)>, Error> {
	let (tx, rx) = oneshot::channel();
	sender
		.send_message(ProspectiveParachainsMessage::GetBackableCandidates(
			relay_parent,
			para_id,
			count,
			ancestors,
			tx,
		))
		.await;

	rx.await.map_err(Error::CanceledBackableCandidates)
}

/// The availability bitfield for a given core is the transpose
/// of a set of signed availability bitfields. It goes like this:
///
/// - construct a transverse slice along `core_idx`
/// - bitwise-or it with the availability slice
/// - count the 1 bits, compare to the total length; true on 2/3+
fn bitfields_indicate_availability(
	core_idx: usize,
	bitfields: &[SignedAvailabilityBitfield],
	availability: &CoreAvailability,
) -> bool {
	let mut availability = availability.clone();
	let availability_len = availability.len();

	for bitfield in bitfields {
		let validator_idx = bitfield.validator_index().0 as usize;
		match availability.get_mut(validator_idx) {
			None => {
				// in principle, this function might return a `Result<bool, Error>` so that we can
				// more clearly express this error condition however, in practice, that would just
				// push off an error-handling routine which would look a whole lot like this one.
				// simpler to just handle the error internally here.
				gum::warn!(
					target: LOG_TARGET,
					validator_idx = %validator_idx,
					availability_len = %availability_len,
					"attempted to set a transverse bit at idx {} which is greater than bitfield size {}",
					validator_idx,
					availability_len,
				);

				return false
			},
			Some(mut bit_mut) => *bit_mut |= bitfield.payload().0[core_idx],
		}
	}

	3 * availability.count_ones() >= 2 * availability.len()
}
