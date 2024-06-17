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

use frame_support::ensure;
use parity_scale_codec::Encode;
use sp_runtime::app_crypto::sp_core;
use sp_std::prelude::*;
use std::collections::HashSet;
use xcm::latest::prelude::*;
use xcm_executor::traits::TransferType;

pub enum Error {
	DuplicateAsset,
	FeesNotMet,
	IncorrectSourceContext,
	UnsupportedTransferType,
}

// /// XCM program to be run on `dst_chain` when transferring `assets` from `src_context` chain.
// ///
// /// All parameters should be specified as anchored to the context of `src_context` (as the source
// /// chain sees them). This function will reanchor them to the context of `dst_chain`.
// ///
// /// Specifying `fees` will first receive the fees asset, then use it to `BuyExecution` before
// /// receiving the other assets.
// /// Fees still have to be part of `assets` list, the `fees` parameter only identifies which one
// and /// how much of it should be used for `BuyExecution`.
// ///
// /// Note: The program returned by this function is not complete on its own. When run on
// `dst_chain`, /// it only handles receiving the `assets` from `src_context` chain and loading them
// into its /// *holding register*, then clears origin.
// /// What happens with the assets after being received is left to the control of the caller. The
// /// caller take the XCM returned by this function and append to it further instructions that do
// /// something with the received assets.
// pub fn transfer_assets_destination_program(
// 	src_context: InteriorLocation,
// 	dst_chain: Location,
// 	assets: Vec<(Asset, TransferType)>,
// 	fees: Option<(AssetId, u128)>,
// ) -> Result<Xcm<()>, Error> {
// 	ensure!(src_context.global_consensus().is_ok(), Error::IncorrectSourceContext);
//
// 	let mut buy_execution = vec![];
// 	let mut teleports = vec![];
// 	let mut reserve_deposits = vec![];
// 	let mut reserve_withdrawals = vec![];
//
// 	for (asset, transfer_type) in assets.into_iter() {
// 		// TODO: do input validations:
// 		// - no asset duplicate
// 		// - asset has unique TransferType
// 		// - no "remote-reserve" transfer types allowed
//
// 		if let Some((fees_id, fees_amount)) = fees.as_ref() {
// 			if asset.id.eq(fees_id) {
// 				let fees = (fees_id, fees_amount).into();
//
// 				buy_execution.push(match transfer_type {
// 					TransferType::Teleport => BuyExecution { fees, weight_limit: Unlimited },
// 					TransferType::LocalReserve => BuyExecution { fees, weight_limit: Unlimited },
// 					TransferType::DestinationReserve =>
// 						BuyExecution { fees, weight_limit: Unlimited },
// 					TransferType::RemoteReserve(_) => unreachable!(),
// 				});
//
// 				buy_execution.push(BuyExecution { fees, weight_limit: Unlimited });
//
// 				continue;
// 			}
// 		}
// 	}
//
// 	ensure!(!buy_execution.is_empty() || fees.is_none(), Error::FeesNotMet);
//
// 	buy_execution.into()
// }

const ASSET_HUB_PARAID: u32 = 1000;
const MOONBEAM_PARAID: u32 = 2004;
const HYDRA_PARAID: u32 = 2034;

pub enum AmountFilter {
	Definite(u128),
	All,
}

fn example_hydra_to_ah_to_moonbeam() {
	// needs to start with `GlobalConsensus`
	let hydra_context = InteriorLocation::X2(GlobalConsensus(Polkadot), Parachain(HYDRA_PARAID));

	// used assets IDs as seen by the `hydra_context` chain
	let hdx_id = AssetId(Here.into());
	let usdt_id = AssetId(Location::new(
		1,
		X3(Parachain(ASSET_HUB_PARAID), PalletInstance(50), GeneralIndex(1984)),
	));
	let glmr_id = AssetId(Location::new(1, X1(Parachain(MOONBEAM_PARAID))));

	// Initialize the builder by defining the starting context and,
	let asset_transfer_builder = AssetTransferBuilder::using_context(hydra_context)
		// registering easy-to-use tags for the assets to transfer - caller can use these tags to
		// easily identify the assets without having to worry about reanchoring and contexts
		.define_asset("HDX", hdx_id)
		.define_asset("USDT", usdt_id)
		.define_asset("GLMR", glmr_id)
		// create the builder
		.create();

	let xcm = asset_transfer_builder
		// the starting context is Hydration Network
		// withdraw assets to transfer from origin account
		.withdraw("HDX", Definite(hdx_amount))
		.withdraw("USDT", Definite(usdt_amount))
		.withdraw("GLMR", Definite(glmr_amount))
		// set AssetHub as the destination for this leg of the transfer
		.set_next_hop(Location::new(1, X1(Parachain(ASSET_HUB_PARAID))))
		// teleport all HDX to Asset Hub
		.transfer("HDX", All, Teleport)
		// reserve-withdraw all USDT on Asset Hub
		.transfer("USDT", All, ReserveWithdraw)
		// reserve-withdraw all GLMR on Asset Hub
		.transfer("GLMR", All, ReserveWithdraw)
		// use USDT to pay for fees on Asset Hub (can define upper limit)
		.pay_remote_fess_with("USDT", Definite(max_usdt_to_use_for_fees))
		// "execute" current leg of the transfer, move to next hop (Asset Hub)
		.execute_hop()
		// from here on, context is Asset Hub
		// set Moonbeam as the destination for this (final) leg of the transfer
		.set_next_hop(Location::new(1, X1(Parachain(MOONBEAM_PARAID))))
		// reserve-deposit HDX to Moonbeam (note we don't need to worry about reanchoring in the new
		// context)
		.transfer("HDX", All, ReserveDeposit)
		// reserve-deposit USDT to Moonbeam (asset reanchoring done behind the scenes)
		.transfer("USDT", All, ReserveDeposit)
		// teleport GLMR to Moonbeam (asset reanchoring done behind the scenes)
		.transfer("GLMR", All, Teleport)
		// use GLMR to pay for fees on Moonbeam (no limit)
		.pay_remote_fess_with("GLMR", All)
		// "execute" current leg of the transfer, move to next hop (Moonbeam)
		.execute_hop()
		// from here on, context is Moonbeam
		// deposit all received assets to `beneficiary`
		.deposit_all(beneficiary)
		// build the asset transfer XCM Program!
		.finalize();

	// Profit!
	println!("Asset transfer XCM: {:?}", xcm);
}

fn example_karura_to_acala() {
	// needs to start with `GlobalConsensus`
	let karura_context = InteriorLocation::X2(GlobalConsensus(Kusama), Parachain(KARURA_PARAID));

	// used assets IDs as seen by the `karura_context` chain
	let ksm_id = AssetId(Location::new(1, Here));

	// Initialize the builder by defining the starting context and,
	let asset_transfer_builder = AssetTransferBuilder::using_context(karura_context)
		// registering easy-to-use tags for the assets to transfer - caller can use these tags to
		// easily identify the assets without having to worry about reanchoring and contexts
		.define_asset("KSM", ksm_id)
		// create the builder
		.create();

	let xcm = asset_transfer_builder
		// the starting context is Karura
		// withdraw assets to transfer from origin account
		.withdraw("KSM", Definite(ksm_amount))
		// set Kusama AssetHub as the destination for this leg of the transfer
		.set_next_hop(Location::new(1, X1(Parachain(KUSAMA_ASSET_HUB_PARAID))))
		// reserve-withdraw all KSM on Asset Hub
		.transfer("KSM", All, ReserveWithdraw)
		// use KSM to pay for fees on Kusama Asset Hub (no limit)
		.pay_remote_fess_with("KSM", All)
		// "execute" current leg of the transfer, move to next hop (Kusama Asset Hub)
		.execute_hop()
		// from here on, context is Kusama Asset Hub
		// set Polkadot Asset Hub as the destination for this leg of the transfer
		.set_next_hop(Location::new(
			2,
			X2(GlobalConsensus(Polkadot), Parachain(POLKADOT_ASSET_HUB_PARAID)),
		))
		// reserve-deposit KSM to Polkadot Asset Hub (asset reanchoring done behind the scenes)
		.transfer("KSM", All, ReserveDeposit)
		// use KSM to pay for fees on Polkadot Asset Hub (no limit)
		.pay_remote_fess_with("KSM", All)
		// "execute" current leg of the transfer, move to next hop (Polkadot Asset Hub)
		.execute_hop()
		// from here on, context is Polkadot Asset Hub
		// set Acala as the destination for this leg of the transfer
		.set_next_hop(Location::new(1, X1(Parachain(ACALA_PARAID))))
		// reserve-deposit KSM to Acala (asset reanchoring done behind the scenes)
		.transfer("KSM", All, ReserveDeposit)
		// use KSM to pay for fees on Acala (no limit)
		.pay_remote_fess_with("KSM", All)
		// "execute" current leg of the transfer, move to next hop (Acala)
		.execute_hop()
		// from here on, context is Acala
		// deposit all received assets to `beneficiary`
		.deposit_all(beneficiary)
		// build the asset transfer XCM Program!
		.finalize();

	// Profit!
	println!("Asset transfer XCM: {:?}", xcm);
}

pub fn deposit_to_account_id_32(
	assets: AssetFilter,
	beneficiary: sp_core::crypto::AccountId32,
) -> Xcm<()> {
	Xcm(vec![DepositAsset {
		assets,
		beneficiary: AccountId32 { network: None, id: beneficiary.into() }.into(),
	}])
}

pub fn deposit_to_location(assets: AssetFilter, beneficiary: Location) -> Xcm<()> {
	Xcm(vec![DepositAsset { assets, beneficiary }])
}

/// XCM program to be run on the destination chain when transferring `assets` in from some origin
/// source chain.
///
/// All parameters should be specified as anchored to the context of the destination chain (as the
/// source chain sees them).
///
/// Specifying `fees` will first receive the fees asset, then use it to `BuyExecution` before
/// receiving the other assets.
/// Fees still have to be part of `assets` list, the `fees` parameter only identifies which one and
/// how much of it should be used for `BuyExecution`.
///
/// Note: The program returned by this function is not complete on its own. When run on destination
/// chain, it only handles receiving the `assets` from source chain and loading them into its
/// *holding register*, then clears origin.
/// What happens with the assets after being received is left to the control of the caller. The
/// caller should take the XCM returned by this function and append to it further instructions that
/// do something with the received assets.
pub fn receive_assets_program_on_destination(
	assets: Vec<(Asset, TransferType)>,
	fees: Option<(AssetId, u128)>,
) -> Result<Xcm<()>, Error> {
	let mut program = vec![];
	let mut to_teleport = vec![];
	let mut to_reserve_deposit = vec![];
	let mut to_reserve_withdraw = vec![];
	let mut unique_assets = HashSet::new();

	for (asset, transfer_type) in assets.into_iter() {
		ensure!(
			!matches!(transfer_type, TransferType::RemoteReserve(_)),
			Error::UnsupportedTransferType
		);
		ensure!(unique_assets.insert(asset.id.encode()), Error::DuplicateAsset);

		if let Some((fees_id, fees_amount)) = fees.clone() {
			if asset.id.eq(&fees_id) {
				let fees: Asset = (fees_id, fees_amount).into();
				program.push(match transfer_type {
					TransferType::Teleport => ReceiveTeleportedAsset(fees.clone().into()),
					TransferType::LocalReserve => ReserveAssetDeposited(fees.clone().into()),
					TransferType::DestinationReserve => WithdrawAsset(fees.clone().into()),
					TransferType::RemoteReserve(_) => unreachable!(),
				});
				program.push(BuyExecution { fees, weight_limit: Unlimited });
				continue;
			}
		}
		match transfer_type {
			TransferType::Teleport => to_teleport.push(asset),
			TransferType::LocalReserve => to_reserve_deposit.push(asset),
			TransferType::DestinationReserve => to_reserve_withdraw.push(asset),
			TransferType::RemoteReserve(_) => unreachable!(),
		};
	}
	ensure!(!program.is_empty() || fees.is_none(), Error::FeesNotMet);

	// add the reserve transfers
	if !to_reserve_deposit.is_empty() {
		program.push(ReserveAssetDeposited(to_reserve_deposit.into()));
	}
	// add the reserve withdrawals
	if !to_reserve_withdraw.is_empty() {
		program.push(WithdrawAsset(to_reserve_withdraw.into()));
	}
	// add the teleports
	if !to_teleport.is_empty() {
		program.push(ReceiveTeleportedAsset(to_teleport.into()));
	}
	// clear origin for subsequent custom instructions
	program.push(ClearOrigin);

	Ok(Xcm(program))
}
