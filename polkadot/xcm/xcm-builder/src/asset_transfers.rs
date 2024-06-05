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

//! Adapters to work with [`frame_support::traits::fungibles`] through XCM.

use frame_support::{
	ensure,
	traits::{Contains, Get},
};
use sp_runtime::{traits::MaybeEquivalence, Either};
use sp_std::{marker::PhantomData, prelude::*, result};
use xcm::latest::prelude::*;
use xcm_executor::traits::{
	Error as MatchError, MatchesFungibles, MatchesNonFungibles, TransferType,
};

pub enum Error {
	FeesNotMet,
	IncorrectSourceContext,
}

/// XCM program to be run on `dst_chain` when transferring `assets` from `src_context` chain.
///
/// All parameters should be specified as anchored to the context of `src_context` (as the source
/// chain sees them). This function will reanchor them to the context of `dst_chain`.
///
/// Specifying `fees` will first receive the fees asset, then use it to `BuyExecution` before
/// receiving the other assets.
/// Fees still have to be part of `assets` list, the `fees` parameter only identifies which one and
/// how much of it should be used for `BuyExecution`.
///
/// Note: The program returned by this function is not complete on its own. When run on `dst_chain`,
/// it only handles receiving the `assets` from `src_context` chain and loading them into its
/// *holding register*.
/// What happens with the assets after being received is left to the control of the caller. The
/// caller take the XCM returned by this function and append to it further instructions that do
/// something with the received assets.
pub fn transfer_assets_destination_program(
	src_context: InteriorLocation,
	dst_chain: Location,
	assets: Vec<(Asset, TransferType)>,
	fees: Option<(AssetId, u128)>,
) -> Result<Xcm<()>, Error> {
	ensure!(src_context.global_consensus().is_ok(), Error::IncorrectSourceContext);

	let mut buy_execution = vec![];
	let mut teleports = vec![];
	let mut reserve_deposits = vec![];
	let mut reserve_withdrawals = vec![];

	for (asset, transfer_type) in assets.into_iter() {
		// TODO: do input validations:
		// - no asset duplicate
		// - asset has unique TransferType
		// - no "remote-reserve" transfer types allowed

		if let Some((fees_id, fees_amount)) = fees.as_ref() {
			if asset.id.eq(fees_id) {
				let fees = (fees_id, fees_amount).into();

				buy_execution.push(match transfer_type {
					TransferType::Teleport => BuyExecution { fees, weight_limit: Unlimited },
					TransferType::LocalReserve => BuyExecution { fees, weight_limit: Unlimited },
					TransferType::DestinationReserve =>
						BuyExecution { fees, weight_limit: Unlimited },
					TransferType::RemoteReserve(_) => unreachable!(),
				});

				buy_execution.push(BuyExecution { fees, weight_limit: Unlimited });

				continue;
			}
		}
	}

	ensure!(!buy_execution.is_empty() || fees.is_none(), Error::FeesNotMet);

	buy_execution.into()
}

pub fn receive_assets_program_on_destination(
	src_context: InteriorLocation,
	dst_chain: Location,
	assets: Vec<(Asset, TransferType)>,
	fees: Option<(AssetId, u128)>,
) -> Result<Xcm<()>, Error> {
	ensure!(src_context.global_consensus().is_ok(), Error::IncorrectSourceContext);

	let mut buy_execution = vec![];
	let mut teleports = vec![];
	let mut reserve_deposits = vec![];
	let mut reserve_withdrawals = vec![];

	for (asset, transfer_type) in assets.into_iter() {
		// TODO: do input validations:
		// - no asset duplicate
		// - asset has unique TransferType
		// - no "remote-reserve" transfer types allowed

		if let Some((fees_id, fees_amount)) = fees.as_ref() {
			if asset.id.eq(fees_id) {
				let fees = (fees_id, fees_amount).into();

				buy_execution.push(match transfer_type {
					TransferType::Teleport => BuyExecution { fees, weight_limit: Unlimited },
					TransferType::LocalReserve => BuyExecution { fees, weight_limit: Unlimited },
					TransferType::DestinationReserve =>
						BuyExecution { fees, weight_limit: Unlimited },
					TransferType::RemoteReserve(_) => unreachable!(),
				});

				buy_execution.push(BuyExecution { fees, weight_limit: Unlimited });

				continue;
			}
		}
	}

	ensure!(!buy_execution.is_empty() || fees.is_none(), Error::FeesNotMet);

	buy_execution.into()
}
