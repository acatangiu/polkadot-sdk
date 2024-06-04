// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::tests::*;

fn send_asset_from_asset_hub_rococo_to_asset_hub_westend(id: Location, amount: u128) {
	let destination = asset_hub_westend_location();

	// fund the AHR's SA on BHR for paying bridge transport fees
	BridgeHubRococo::fund_para_sovereign(AssetHubRococo::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	AssetHubRococo::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubRococo::force_xcm_version(bridge_hub_westend_location(), XCM_VERSION);

	// send message over bridge
	assert_ok!(send_asset_from_asset_hub_rococo(destination, (id, amount)));
	assert_bridge_hub_rococo_message_accepted(true);
	assert_bridge_hub_westend_message_received();
}

fn send_asset_from_penpal_rococo_through_local_asset_hub_to_westend_asset_hub(
	id: Location,
	transfer_amount: u128,
) {
	let destination = asset_hub_westend_location();
	let local_asset_hub: Location = PenpalA::sibling_location_of(AssetHubRococo::para_id());
	let sov_penpal_on_ahr = AssetHubRococo::sovereign_account_id_of(
		AssetHubRococo::sibling_location_of(PenpalA::para_id()),
	);
	let sov_ahw_on_ahr = AssetHubRococo::sovereign_account_of_parachain_on_other_global_consensus(
		Westend,
		AssetHubWestend::para_id(),
	);

	// fund the AHR's SA on BHR for paying bridge transport fees
	BridgeHubRococo::fund_para_sovereign(AssetHubRococo::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	PenpalA::force_xcm_version(local_asset_hub.clone(), XCM_VERSION);
	AssetHubRococo::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubRococo::force_xcm_version(bridge_hub_westend_location(), XCM_VERSION);

	// send message over bridge
	assert_ok!(PenpalA::execute_with(|| {
		let signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: AssetHubWestendReceiver::get().into() }.into();
		let assets: Assets = (id.clone(), transfer_amount).into();
		let fees_id: AssetId = id.into();
		let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
			assets: Wild(AllCounted(assets.len() as u32)),
			beneficiary,
		}]);

		<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
			signed_origin,
			bx!(destination.into()),
			bx!(assets.clone().into()),
			bx!(TransferType::RemoteReserve(local_asset_hub.clone().into())),
			bx!(fees_id.into()),
			bx!(TransferType::RemoteReserve(local_asset_hub.into())),
			bx!(VersionedXcm::from(custom_xcm_on_dest)),
			WeightLimit::Unlimited,
		)
	}));
	AssetHubRococo::execute_with(|| {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubRococo,
			vec![
				// Amount to reserve transfer is withdrawn from Penpal's sovereign account
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_penpal_on_ahr.clone().into(),
					amount: *amount == transfer_amount,
				},
				// Amount deposited in AHW's sovereign account
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == sov_ahw_on_ahr.clone().into(),
				},
				RuntimeEvent::XcmpQueue(
					cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	});
	assert_bridge_hub_rococo_message_accepted(true);
	assert_bridge_hub_westend_message_received();
}

#[test]
fn send_rocs_from_asset_hub_rococo_to_asset_hub_westend() {
	let roc_at_asset_hub_rococo: v3::Location = v3::Parent.into();
	let roc_at_asset_hub_westend =
		v3::Location::new(2, [v3::Junction::GlobalConsensus(v3::NetworkId::Rococo)]);
	let owner: AccountId = AssetHubWestend::account_id_of(ALICE);
	AssetHubWestend::force_create_foreign_asset(
		roc_at_asset_hub_westend,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);
	let sov_ahw_on_ahr = AssetHubRococo::sovereign_account_of_parachain_on_other_global_consensus(
		Westend,
		AssetHubWestend::para_id(),
	);

	AssetHubWestend::execute_with(|| {
		type RuntimeEvent = <AssetHubWestend as Chain>::RuntimeEvent;

		// setup a pool to pay xcm fees with `roc_at_asset_hub_westend` tokens
		assert_ok!(<AssetHubWestend as AssetHubWestendPallet>::ForeignAssets::mint(
			<AssetHubWestend as Chain>::RuntimeOrigin::signed(AssetHubWestendSender::get()),
			roc_at_asset_hub_westend.into(),
			AssetHubWestendSender::get().into(),
			3_000_000_000_000,
		));

		assert_ok!(<AssetHubWestend as AssetHubWestendPallet>::AssetConversion::create_pool(
			<AssetHubWestend as Chain>::RuntimeOrigin::signed(AssetHubWestendSender::get()),
			Box::new(xcm::v3::Parent.into()),
			Box::new(roc_at_asset_hub_westend),
		));

		assert_expected_events!(
			AssetHubWestend,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubWestend as AssetHubWestendPallet>::AssetConversion::add_liquidity(
			<AssetHubWestend as Chain>::RuntimeOrigin::signed(AssetHubWestendSender::get()),
			Box::new(xcm::v3::Parent.into()),
			Box::new(roc_at_asset_hub_westend),
			1_000_000_000_000,
			2_000_000_000_000,
			1,
			1,
			AssetHubWestendSender::get().into()
		));

		assert_expected_events!(
			AssetHubWestend,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {..}) => {},
			]
		);
	});

	let rocs_in_reserve_on_ahr_before =
		<AssetHubRococo as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let sender_rocs_before =
		<AssetHubRococo as Chain>::account_data_of(AssetHubRococoSender::get()).free;
	let receiver_rocs_before = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(roc_at_asset_hub_westend, &AssetHubWestendReceiver::get())
	});

	let amount = ASSET_HUB_ROCOCO_ED * 1_000_000;
	send_asset_from_asset_hub_rococo_to_asset_hub_westend(
		roc_at_asset_hub_rococo.try_into().unwrap(),
		amount,
	);
	AssetHubWestend::execute_with(|| {
		type RuntimeEvent = <AssetHubWestend as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubWestend,
			vec![
				// issue ROCs on AHW
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == roc_at_asset_hub_rococo,
					owner: *owner == AssetHubWestendReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_rocs_after =
		<AssetHubRococo as Chain>::account_data_of(AssetHubRococoSender::get()).free;
	let receiver_rocs_after = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(roc_at_asset_hub_westend, &AssetHubWestendReceiver::get())
	});
	let rocs_in_reserve_on_ahr_after =
		<AssetHubRococo as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;

	// Sender's balance is reduced
	assert!(sender_rocs_before > sender_rocs_after);
	// Receiver's balance is increased
	assert!(receiver_rocs_after > receiver_rocs_before);
	// Reserve balance is increased by sent amount
	assert_eq!(rocs_in_reserve_on_ahr_after, rocs_in_reserve_on_ahr_before + amount);
}

#[test]
fn send_wnds_from_asset_hub_rococo_to_asset_hub_westend() {
	let prefund_amount = 10_000_000_000_000u128;
	let wnd_at_asset_hub_rococo =
		v3::Location::new(2, [v3::Junction::GlobalConsensus(v3::NetworkId::Westend)]);
	let owner: AccountId = AssetHubRococo::account_id_of(ALICE);
	AssetHubRococo::force_create_foreign_asset(
		wnd_at_asset_hub_rococo,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![(AssetHubRococoSender::get(), prefund_amount)],
	);

	// fund the AHR's SA on AHW with the WND tokens held in reserve
	let sov_ahr_on_ahw = AssetHubWestend::sovereign_account_of_parachain_on_other_global_consensus(
		Rococo,
		AssetHubRococo::para_id(),
	);
	AssetHubWestend::fund_accounts(vec![(sov_ahr_on_ahw.clone(), prefund_amount)]);

	let wnds_in_reserve_on_ahw_before =
		<AssetHubWestend as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	assert_eq!(wnds_in_reserve_on_ahw_before, prefund_amount);
	let sender_wnds_before = AssetHubRococo::execute_with(|| {
		type Assets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(wnd_at_asset_hub_rococo, &AssetHubRococoSender::get())
	});
	assert_eq!(sender_wnds_before, prefund_amount);
	let receiver_wnds_before =
		<AssetHubWestend as Chain>::account_data_of(AssetHubWestendReceiver::get()).free;

	let amount_to_send = ASSET_HUB_WESTEND_ED * 1_000;
	send_asset_from_asset_hub_rococo_to_asset_hub_westend(
		Location::try_from(wnd_at_asset_hub_rococo).unwrap(),
		amount_to_send,
	);
	AssetHubWestend::execute_with(|| {
		type RuntimeEvent = <AssetHubWestend as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubWestend,
			vec![
				// WND is withdrawn from AHR's SA on AHW
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_ahr_on_ahw,
					amount: *amount == amount_to_send,
				},
				// WNDs deposited to beneficiary
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == AssetHubWestendReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_wnds_after = AssetHubRococo::execute_with(|| {
		type Assets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(wnd_at_asset_hub_rococo, &AssetHubRococoSender::get())
	});
	let receiver_wnds_after =
		<AssetHubWestend as Chain>::account_data_of(AssetHubWestendReceiver::get()).free;
	let wnds_in_reserve_on_ahw_after =
		<AssetHubWestend as Chain>::account_data_of(sov_ahr_on_ahw).free;

	// Sender's balance is reduced
	assert!(sender_wnds_before > sender_wnds_after);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(wnds_in_reserve_on_ahw_after, wnds_in_reserve_on_ahw_before - amount_to_send);
}

#[test]
fn send_rocs_from_penpal_rococo_through_asset_hub_rococo_to_asset_hub_westend() {
	let roc_at_rococo_parachains: Location = Parent.into();
	let roc_at_asset_hub_westend = Location::new(2, [Junction::GlobalConsensus(NetworkId::Rococo)]);
	let owner: AccountId = AssetHubWestend::account_id_of(ALICE);
	AssetHubWestend::force_create_foreign_asset(
		roc_at_asset_hub_westend.clone().try_into().unwrap(),
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);
	let sov_ahw_on_ahr = AssetHubRococo::sovereign_account_of_parachain_on_other_global_consensus(
		Westend,
		AssetHubWestend::para_id(),
	);

	let amount = ASSET_HUB_ROCOCO_ED * 10_000_000;
	let penpal_location = AssetHubRococo::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubRococo::sovereign_account_id_of(penpal_location);
	// fund Penpal's sovereign account on AssetHub
	AssetHubRococo::fund_accounts(vec![(sov_penpal_on_ahr.into(), amount * 2)]);
	// fund Penpal's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		roc_at_rococo_parachains.clone(),
		PenpalASender::get(),
		amount * 2,
	);

	let rocs_in_reserve_on_ahr_before =
		<AssetHubRococo as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let sender_rocs_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			roc_at_rococo_parachains.clone(),
			&PenpalASender::get(),
		)
	});
	let receiver_rocs_before = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			roc_at_asset_hub_westend.clone().try_into().unwrap(),
			&AssetHubWestendReceiver::get(),
		)
	});
	send_asset_from_penpal_rococo_through_local_asset_hub_to_westend_asset_hub(
		roc_at_rococo_parachains.clone(),
		amount,
	);

	AssetHubWestend::execute_with(|| {
		type RuntimeEvent = <AssetHubWestend as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubWestend,
			vec![
				// issue ROCs on AHW
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == roc_at_rococo_parachains.clone().try_into().unwrap(),
					owner: *owner == AssetHubWestendReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_rocs_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_at_rococo_parachains, &PenpalASender::get())
	});
	let receiver_rocs_after = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			roc_at_asset_hub_westend.try_into().unwrap(),
			&AssetHubWestendReceiver::get(),
		)
	});
	let rocs_in_reserve_on_ahr_after =
		<AssetHubRococo as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;

	// Sender's balance is reduced
	assert!(sender_rocs_after < sender_rocs_before);
	// Receiver's balance is increased
	assert!(receiver_rocs_after > receiver_rocs_before);
	// Reserve balance is increased by sent amount (less fess)
	assert!(rocs_in_reserve_on_ahr_after > rocs_in_reserve_on_ahr_before);
	assert!(rocs_in_reserve_on_ahr_after <= rocs_in_reserve_on_ahr_before + amount);
}

fn do_send_pens_and_rocs_from_penpal_rococo_via_ahr_to_ahw(
	rocs: (Location, u128),
	pens: (Location, u128),
) {
	let (rocs_id, rocs_amount) = rocs;
	let (pens_id, pens_amount) = pens;
	let destination = asset_hub_westend_location();
	let local_asset_hub: Location = PenpalA::sibling_location_of(AssetHubRococo::para_id());
	let sov_penpal_on_ahr = AssetHubRococo::sovereign_account_id_of(
		AssetHubRococo::sibling_location_of(PenpalA::para_id()),
	);
	let sov_ahw_on_ahr = AssetHubRococo::sovereign_account_of_parachain_on_other_global_consensus(
		Westend,
		AssetHubWestend::para_id(),
	);

	// fund the AHR's SA on BHR for paying bridge transport fees
	BridgeHubRococo::fund_para_sovereign(AssetHubRococo::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	PenpalA::force_xcm_version(local_asset_hub.clone(), XCM_VERSION);
	AssetHubRococo::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubRococo::force_xcm_version(bridge_hub_westend_location(), XCM_VERSION);

	// send message over bridge
	assert_ok!(PenpalA::execute_with(|| {
		let signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: AssetHubWestendReceiver::get().into() }.into();
		let rocs: Asset = (rocs_id.clone(), rocs_amount).into();
		let pens: Asset = (pens_id, pens_amount).into();
		let assets: Assets = vec![rocs.clone(), pens.clone()].into();

		// XCM to be executed at dest (Westend Asset Hub)
		let xcm_on_dest =
			Xcm(vec![DepositAsset { assets: Wild(All), beneficiary: beneficiary.clone() }]);

		// XCM to be executed at Rococo Asset Hub
		let context = PenpalUniversalLocation::get();
		let reanchored_assets = assets.clone().reanchored(&local_asset_hub, &context).unwrap();
		let reanchored_dest = destination.clone().reanchored(&local_asset_hub, &context).unwrap();
		let reanchored_rocs_id = rocs_id.clone().reanchored(&local_asset_hub, &context).unwrap();
		let fun = WildFungibility::Fungible;
		let xcm_on_ahr = Xcm(vec![
			// both ROCs and PENs are local-reserve transferred to Westend Asset Hub
			LocalReserveDepositAssets(reanchored_assets.clone().into()),
			ExecuteAssetTransfers {
				dest: reanchored_dest,
				remote_fees: Some(AssetFilter::Wild(AllOf { id: reanchored_rocs_id.into(), fun })),
				remote_xcm: xcm_on_dest,
			},
		]);

		// XCM to be executed locally
		let xcm = Xcm::<penpal_runtime::RuntimeCall>(vec![
			// Withdraw both ROCs and PENs from origin account
			WithdrawAsset(assets.clone().into()),
			// ROCs are reserve-withdrawn on AHR
			DestinationReserveWithdrawAssets(rocs.into()),
			// PENs are teleported to AHR
			TeleportTransferAssets(pens.into()),
			// Execute the transfers while paying remote fees with ROCs
			ExecuteAssetTransfers {
				dest: local_asset_hub,
				remote_fees: Some(AssetFilter::Wild(AllOf { id: rocs_id.into(), fun })),
				remote_xcm: xcm_on_ahr,
			},
		]);

		println!("💰💘🤑 PenpalA execute {xcm:?}");

		<PenpalA as PenpalAPallet>::PolkadotXcm::execute(
			signed_origin,
			bx!(xcm::VersionedXcm::V4(xcm.into())),
			Weight::MAX,
		)
	}));
	AssetHubRococo::execute_with(|| {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubRococo,
			vec![
				// Amount to reserve transfer is withdrawn from Penpal's sovereign account
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_penpal_on_ahr.clone().into(),
					amount: *amount == rocs_amount,
				},
				// Amount deposited in AHW's sovereign account
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == sov_ahw_on_ahr.clone().into(),
				},
				RuntimeEvent::XcmpQueue(
					cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	});
	assert_bridge_hub_rococo_message_accepted(true);
	assert_bridge_hub_westend_message_received();
}

/// Transfers of penpal "PEN"s plus "ROC"s from PenpalRococo to AssetHubRococo,
/// over bridge to AssetHubWestend. Where PENs need to be teleported to AHR, while ROCs
/// reserve-withdrawn, then both reserve transferred further to AHW.
/// (transfer 2 different assets with different transfer types across 3 different chains)
#[test]
fn send_pens_and_rocs_from_penpal_rococo_via_ahr_to_ahw() {
	let penpal_check_account = <PenpalA as PenpalAPallet>::PolkadotXcm::check_account();
	let owner: AccountId = AssetHubWestend::account_id_of(ALICE);
	let sender = PenpalASender::get();

	let roc_at_rococo_parachains: Location = Parent.into();
	let roc_at_westend_parachains =
		v3::Location::new(2, [v3::Junction::GlobalConsensus(v3::NetworkId::Rococo)]);

	let pens_location_on_penpal =
		v3::Location::try_from(PenpalLocalTeleportableToAssetHub::get()).unwrap();
	let pens_id_on_penpal = match pens_location_on_penpal.last() {
		Some(v3::Junction::GeneralIndex(id)) => *id as u32,
		_ => unreachable!(),
	};

	let penpal_parachain_junction = v3::Junction::Parachain(PenpalA::para_id().into());
	let pens_at_ahr = v3::Location::new(
		1,
		pens_location_on_penpal
			.interior()
			.clone()
			.pushed_front_with(penpal_parachain_junction.clone())
			.unwrap(),
	);
	println!("🤡 pens_at_ahr {pens_at_ahr:?}");
	let pens_at_westend_parachains = v3::Location::new(
		2,
		pens_at_ahr
			.interior()
			.clone()
			.pushed_front_with(v3::Junction::GlobalConsensus(v3::NetworkId::Rococo))
			.unwrap(),
	);
	println!("🤡 pens_at_westend_parachains {pens_at_westend_parachains:?}");
	let rocs_to_send = ASSET_HUB_ROCOCO_ED * 10_000_000;
	let pens_to_send = ASSET_HUB_ROCOCO_ED * 10_000_000;

	// ---------- Set up Penpal Rococo ----------
	// fund Penpal's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(owner.clone()),
		roc_at_rococo_parachains.clone(),
		sender.clone(),
		rocs_to_send * 2,
	);
	// No need to create the asset (only mint) as it exists in genesis.
	PenpalA::mint_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(owner.clone()),
		pens_id_on_penpal,
		sender.clone(),
		pens_to_send * 2,
	);
	// fund Penpal's check account to be able to teleport
	PenpalA::fund_accounts(vec![(penpal_check_account.clone().into(), pens_to_send * 2)]);

	// ---------- Set up Asset Hub Rococo ----------
	// PENs already created at AHR
	// prefund SA of Penpal on AHR with ROCs to be withdrawn
	let penpal_as_seen_by_ahr = AssetHubRococo::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubRococo::sovereign_account_id_of(penpal_as_seen_by_ahr);
	AssetHubRococo::fund_accounts(vec![(sov_penpal_on_ahr.clone().into(), rocs_to_send * 2)]);
	let sov_ahw_on_ahr = AssetHubRococo::sovereign_account_of_parachain_on_other_global_consensus(
		Westend,
		AssetHubWestend::para_id(),
	);

	// ---------- Set up Asset Hub Westend ----------
	println!("🤡 try create ROC {roc_at_westend_parachains:?} at AHW");
	// create ROC at AHW
	AssetHubWestend::force_create_foreign_asset(
		roc_at_westend_parachains,
		owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);
	println!("🤡 try create PEN {pens_at_westend_parachains:?} at AHW");
	// create PEN at AHW
	AssetHubWestend::force_create_foreign_asset(
		pens_at_westend_parachains,
		owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);

	// account balances before
	let sender_rocs_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			roc_at_rococo_parachains.clone().into(),
			&PenpalASender::get(),
		)
	});
	let sender_pens_before = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::Assets;
		<Assets as Inspect<_>>::balance(pens_id_on_penpal, &PenpalASender::get())
	});
	let rocs_in_reserve_on_ahr_before =
		<AssetHubRococo as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let pens_in_reserve_on_ahr_before = AssetHubRococo::execute_with(|| {
		type ForeignAssets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(pens_at_ahr, &sov_ahw_on_ahr)
	});
	let receiver_rocs_before = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(roc_at_westend_parachains, &AssetHubWestendReceiver::get())
	});
	let receiver_pens_before = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(pens_at_westend_parachains, &AssetHubWestendReceiver::get())
	});

	// transfer assets
	do_send_pens_and_rocs_from_penpal_rococo_via_ahr_to_ahw(
		(roc_at_rococo_parachains.clone(), rocs_to_send),
		(pens_location_on_penpal.try_into().unwrap(), pens_to_send),
	);

	AssetHubWestend::execute_with(|| {
		type RuntimeEvent = <AssetHubWestend as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubWestend,
			vec![
				// issue ROCs on AHW
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == roc_at_rococo_parachains.clone().try_into().unwrap(),
					owner: *owner == AssetHubWestendReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	// account balances after
	let sender_rocs_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			roc_at_rococo_parachains.into(),
			&PenpalASender::get(),
		)
	});
	let sender_pens_after = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::Assets;
		<Assets as Inspect<_>>::balance(pens_id_on_penpal, &PenpalASender::get())
	});
	let rocs_in_reserve_on_ahr_after =
		<AssetHubRococo as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let pens_in_reserve_on_ahr_after = AssetHubRococo::execute_with(|| {
		type ForeignAssets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(pens_at_ahr, &sov_ahw_on_ahr)
	});
	let receiver_rocs_after = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(roc_at_westend_parachains, &AssetHubWestendReceiver::get())
	});
	let receiver_pens_after = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(pens_at_westend_parachains, &AssetHubWestendReceiver::get())
	});

	println!("🤡 sender rocs before {:?} after {:?}", sender_rocs_before, sender_rocs_after);
	println!(
		"🤡 in AHR reserve rocs before {:?} after {:?}",
		rocs_in_reserve_on_ahr_before, rocs_in_reserve_on_ahr_after
	);
	println!("🤡 receiver rocs before {:?} after {:?}", receiver_rocs_before, receiver_rocs_after);

	println!("🤡 sender pens before {:?} after {:?}", sender_pens_before, sender_pens_after);
	println!(
		"🤡 in AHR reserve pens before {:?} after {:?}",
		pens_in_reserve_on_ahr_before, pens_in_reserve_on_ahr_after
	);
	println!("🤡 receiver pens before {:?} after {:?}", receiver_pens_before, receiver_pens_after);

	// Sender's balance is reduced
	assert!(sender_rocs_after < sender_rocs_before);
	// Receiver's balance is increased
	assert!(receiver_rocs_after > receiver_rocs_before);
	// Reserve balance is increased by sent amount (less fess)
	assert!(rocs_in_reserve_on_ahr_after > rocs_in_reserve_on_ahr_before);
	assert!(rocs_in_reserve_on_ahr_after <= rocs_in_reserve_on_ahr_before + rocs_to_send);

	// Sender's balance is reduced by sent amount
	assert_eq!(sender_pens_after, sender_pens_before - pens_to_send);
	// Reserve balance is increased by sent amount
	assert_eq!(pens_in_reserve_on_ahr_after, pens_in_reserve_on_ahr_before + pens_to_send);
	// Receiver's balance is increased by sent amount
	assert_eq!(receiver_pens_after, receiver_pens_before + pens_to_send);
}
