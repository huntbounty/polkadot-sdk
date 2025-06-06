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

use super::reserve_transfer::*;
use crate::{
	imports::*,
	tests::teleport::do_bidirectional_teleport_foreign_assets_between_para_and_asset_hub_using_xt,
};

fn para_to_para_assethub_hop_assertions(t: ParaToParaThroughAHTest) {
	type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;
	let sov_penpal_a_on_ah = AssetHubRococo::sovereign_account_id_of(
		AssetHubRococo::sibling_location_of(PenpalA::para_id()),
	);
	let sov_penpal_b_on_ah = AssetHubRococo::sovereign_account_id_of(
		AssetHubRococo::sibling_location_of(PenpalB::para_id()),
	);

	assert_expected_events!(
		AssetHubRococo,
		vec![
			// Withdrawn from sender parachain SA
			RuntimeEvent::Balances(
				pallet_balances::Event::Burned { who, amount }
			) => {
				who: *who == sov_penpal_a_on_ah,
				amount: *amount == t.args.amount,
			},
			// Deposited to receiver parachain SA
			RuntimeEvent::Balances(
				pallet_balances::Event::Minted { who, .. }
			) => {
				who: *who == sov_penpal_b_on_ah,
			},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn ah_to_para_transfer_assets(t: SystemParaToParaTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<AssetHubRococo as AssetHubRococoPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::LocalReserve),
		bx!(fee.id.into()),
		bx!(TransferType::LocalReserve),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

fn para_to_ah_transfer_assets(t: ParaToSystemParaTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::DestinationReserve),
		bx!(fee.id.into()),
		bx!(TransferType::DestinationReserve),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

fn para_to_para_transfer_assets_through_ah(t: ParaToParaThroughAHTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let asset_hub_location: Location = PenpalA::sibling_location_of(AssetHubRococo::para_id());
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::RemoteReserve(asset_hub_location.clone().into())),
		bx!(fee.id.into()),
		bx!(TransferType::RemoteReserve(asset_hub_location.into())),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

fn para_to_asset_hub_teleport_foreign_assets(t: ParaToSystemParaTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::Teleport),
		bx!(fee.id.into()),
		bx!(TransferType::DestinationReserve),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

fn asset_hub_to_para_teleport_foreign_assets(t: SystemParaToParaTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<AssetHubRococo as AssetHubRococoPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::Teleport),
		bx!(fee.id.into()),
		bx!(TransferType::LocalReserve),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

// ===========================================================================
// ======= Transfer - Native + Bridged Assets - AssetHub->Parachain ==========
// ===========================================================================
/// Transfers of native asset plus bridged asset from AssetHub to some Parachain
/// while paying fees using native asset.
#[test]
fn transfer_foreign_assets_from_asset_hub_to_para() {
	let destination = AssetHubRococo::sibling_location_of(PenpalA::para_id());
	let sender = AssetHubRococoSender::get();
	let native_amount_to_send: Balance = ASSET_HUB_ROCOCO_ED * 10000;
	let native_asset_location = RelayLocation::get();
	let receiver = PenpalAReceiver::get();
	let assets_owner = PenpalAssetOwner::get();
	// Foreign asset used: bridged WND
	let foreign_amount_to_send = ASSET_HUB_ROCOCO_ED * 10_000_000;
	let wnd_at_rococo_parachains =
		Location::new(2, [Junction::GlobalConsensus(NetworkId::ByGenesis(WESTEND_GENESIS_HASH))]);

	// Configure destination chain to trust AH as reserve of WND
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(ByGenesis(WESTEND_GENESIS_HASH))]).encode(),
			)],
		));
	});
	PenpalA::force_create_foreign_asset(
		wnd_at_rococo_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubRococo::force_create_foreign_asset(
		wnd_at_rococo_parachains.clone().try_into().unwrap(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubRococo::mint_foreign_asset(
		<AssetHubRococo as Chain>::RuntimeOrigin::signed(assets_owner),
		wnd_at_rococo_parachains.clone().try_into().unwrap(),
		sender.clone(),
		foreign_amount_to_send * 2,
	);

	// Assets to send
	let assets: Vec<Asset> = vec![
		(Parent, native_amount_to_send).into(),
		(wnd_at_rococo_parachains.clone(), foreign_amount_to_send).into(),
	];
	let fee_asset_id = AssetId(Parent.into());
	let fee_asset_item = assets.iter().position(|a| a.id == fee_asset_id).unwrap() as u32;

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination.clone(),
			receiver.clone(),
			native_amount_to_send,
			assets.into(),
			None,
			fee_asset_item,
		),
	};
	let mut test = SystemParaToParaTest::new(test_args);

	// Query initial balances
	let sender_balance_before = test.sender.balance;
	let sender_wnds_before = AssetHubRococo::execute_with(|| {
		type ForeignAssets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			wnd_at_rococo_parachains.clone().try_into().unwrap(),
			&sender,
		)
	});
	let receiver_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location.clone(), &receiver)
	});
	let receiver_wnds_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_rococo_parachains.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<AssetHubRococo>(system_para_to_para_sender_assertions);
	test.set_assertion::<PenpalA>(system_para_to_para_receiver_assertions);
	test.set_dispatchable::<AssetHubRococo>(ah_to_para_transfer_assets);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let sender_wnds_after = AssetHubRococo::execute_with(|| {
		type ForeignAssets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			wnd_at_rococo_parachains.clone().try_into().unwrap(),
			&sender,
		)
	});
	let receiver_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location, &receiver)
	});
	let receiver_wnds_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_rococo_parachains, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_balance_after < sender_balance_before - native_amount_to_send);
	// Sender's balance is reduced by foreign amount sent
	assert_eq!(sender_wnds_after, sender_wnds_before - foreign_amount_to_send);
	// Receiver's assets is increased
	assert!(receiver_assets_after > receiver_assets_before);
	// Receiver's assets increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_assets_after < receiver_assets_before + native_amount_to_send);
	// Receiver's balance is increased by foreign amount sent
	assert_eq!(receiver_wnds_after, receiver_wnds_before + foreign_amount_to_send);
}

/// Reserve Transfers of native asset from Parachain to System Parachain should work
// ===========================================================================
// ======= Transfer - Native + Bridged Assets - Parachain->AssetHub ==========
// ===========================================================================
/// Transfers of native asset plus bridged asset from some Parachain to AssetHub
/// while paying fees using native asset.
#[test]
fn transfer_foreign_assets_from_para_to_asset_hub() {
	// Init values for Parachain
	let destination = PenpalA::sibling_location_of(AssetHubRococo::para_id());
	let sender = PenpalASender::get();
	let native_amount_to_send: Balance = ASSET_HUB_ROCOCO_ED * 10000;
	let native_asset_location = RelayLocation::get();
	let assets_owner = PenpalAssetOwner::get();

	// Foreign asset used: bridged WND
	let foreign_amount_to_send = ASSET_HUB_ROCOCO_ED * 10_000_000;
	let wnd_at_rococo_parachains =
		Location::new(2, [Junction::GlobalConsensus(NetworkId::ByGenesis(WESTEND_GENESIS_HASH))]);

	// Configure destination chain to trust AH as reserve of WND
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(ByGenesis(WESTEND_GENESIS_HASH))]).encode(),
			)],
		));
	});
	PenpalA::force_create_foreign_asset(
		wnd_at_rococo_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubRococo::force_create_foreign_asset(
		wnd_at_rococo_parachains.clone().try_into().unwrap(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);

	// fund Parachain's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		native_asset_location.clone(),
		sender.clone(),
		native_amount_to_send * 2,
	);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		wnd_at_rococo_parachains.clone(),
		sender.clone(),
		foreign_amount_to_send * 2,
	);

	// Init values for System Parachain
	let receiver = AssetHubRococoReceiver::get();
	let penpal_location_as_seen_by_ahr = AssetHubRococo::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubRococo::sovereign_account_id_of(penpal_location_as_seen_by_ahr);

	// fund Parachain's SA on AssetHub with the assets held in reserve
	AssetHubRococo::fund_accounts(vec![(
		sov_penpal_on_ahr.clone().into(),
		native_amount_to_send * 2,
	)]);
	AssetHubRococo::mint_foreign_asset(
		<AssetHubRococo as Chain>::RuntimeOrigin::signed(assets_owner),
		wnd_at_rococo_parachains.clone().try_into().unwrap(),
		sov_penpal_on_ahr,
		foreign_amount_to_send * 2,
	);

	// Assets to send
	let assets: Vec<Asset> = vec![
		(Parent, native_amount_to_send).into(),
		(wnd_at_rococo_parachains.clone(), foreign_amount_to_send).into(),
	];
	let fee_asset_id = AssetId(Parent.into());
	let fee_asset_item = assets.iter().position(|a| a.id == fee_asset_id).unwrap() as u32;

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination.clone(),
			receiver.clone(),
			native_amount_to_send,
			assets.into(),
			None,
			fee_asset_item,
		),
	};
	let mut test = ParaToSystemParaTest::new(test_args);

	// Query initial balances
	let sender_native_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location.clone(), &sender)
	});
	let sender_wnds_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_rococo_parachains.clone(), &sender)
	});
	let receiver_native_before = test.receiver.balance;
	let receiver_wnds_before = AssetHubRococo::execute_with(|| {
		type ForeignAssets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			wnd_at_rococo_parachains.clone().try_into().unwrap(),
			&receiver,
		)
	});

	// Set assertions and dispatchables
	test.set_assertion::<PenpalA>(para_to_system_para_sender_assertions);
	test.set_assertion::<AssetHubRococo>(para_to_system_para_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_ah_transfer_assets);
	test.assert();

	// Query final balances
	let sender_native_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location, &sender)
	});
	let sender_wnds_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_rococo_parachains.clone(), &sender)
	});
	let receiver_native_after = test.receiver.balance;
	let receiver_wnds_after = AssetHubRococo::execute_with(|| {
		type ForeignAssets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			wnd_at_rococo_parachains.try_into().unwrap(),
			&receiver,
		)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_native_after < sender_native_before - native_amount_to_send);
	// Sender's balance is reduced by foreign amount sent
	assert_eq!(sender_wnds_after, sender_wnds_before - foreign_amount_to_send);
	// Receiver's balance is increased
	assert!(receiver_native_after > receiver_native_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_native_after < receiver_native_before + native_amount_to_send);
	// Receiver's balance is increased by foreign amount sent
	assert_eq!(receiver_wnds_after, receiver_wnds_before + foreign_amount_to_send);
}

// ==============================================================================
// ===== Transfer - Native + Bridged Assets - Parachain->AssetHub->Parachain ====
// ==============================================================================
/// Transfers of native asset plus bridged asset from Parachain to Parachain
/// (through AssetHub reserve) with fees paid using native asset.
#[test]
fn transfer_foreign_assets_from_para_to_para_through_asset_hub() {
	// Init values for Parachain Origin
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let sender = PenpalASender::get();
	let roc_to_send: Balance = ROCOCO_ED * 10000;
	let assets_owner = PenpalAssetOwner::get();
	let roc_location = RelayLocation::get();
	let sender_as_seen_by_ah = AssetHubRococo::sibling_location_of(PenpalA::para_id());
	let sov_of_sender_on_ah = AssetHubRococo::sovereign_account_id_of(sender_as_seen_by_ah);
	let receiver_as_seen_by_ah = AssetHubRococo::sibling_location_of(PenpalB::para_id());
	let sov_of_receiver_on_ah = AssetHubRococo::sovereign_account_id_of(receiver_as_seen_by_ah);
	let wnd_to_send = ASSET_HUB_ROCOCO_ED * 10_000_000;

	// Configure source and destination chains to trust AH as reserve of WND
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(ByGenesis(WESTEND_GENESIS_HASH))]).encode(),
			)],
		));
	});
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(ByGenesis(WESTEND_GENESIS_HASH))]).encode(),
			)],
		));
	});

	// Register WND as foreign asset and transfer it around the Rococo ecosystem
	let wnd_at_rococo_parachains =
		Location::new(2, [Junction::GlobalConsensus(NetworkId::ByGenesis(WESTEND_GENESIS_HASH))]);
	AssetHubRococo::force_create_foreign_asset(
		wnd_at_rococo_parachains.clone().try_into().unwrap(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	PenpalA::force_create_foreign_asset(
		wnd_at_rococo_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	PenpalB::force_create_foreign_asset(
		wnd_at_rococo_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);

	// fund Parachain's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		roc_location.clone(),
		sender.clone(),
		roc_to_send * 2,
	);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		wnd_at_rococo_parachains.clone(),
		sender.clone(),
		wnd_to_send * 2,
	);
	// fund the Parachain Origin's SA on Asset Hub with the assets held in reserve
	AssetHubRococo::fund_accounts(vec![(sov_of_sender_on_ah.clone().into(), roc_to_send * 2)]);
	AssetHubRococo::mint_foreign_asset(
		<AssetHubRococo as Chain>::RuntimeOrigin::signed(assets_owner),
		wnd_at_rococo_parachains.clone().try_into().unwrap(),
		sov_of_sender_on_ah.clone(),
		wnd_to_send * 2,
	);

	// Init values for Parachain Destination
	let receiver = PenpalBReceiver::get();

	// Assets to send
	let assets: Vec<Asset> = vec![
		(roc_location.clone(), roc_to_send).into(),
		(wnd_at_rococo_parachains.clone(), wnd_to_send).into(),
	];
	let fee_asset_id: AssetId = roc_location.clone().into();
	let fee_asset_item = assets.iter().position(|a| a.id == fee_asset_id).unwrap() as u32;

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination,
			receiver.clone(),
			roc_to_send,
			assets.into(),
			None,
			fee_asset_item,
		),
	};
	let mut test = ParaToParaThroughAHTest::new(test_args);

	// Query initial balances
	let sender_rocs_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_location.clone(), &sender)
	});
	let sender_wnds_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_rococo_parachains.clone(), &sender)
	});
	let rocs_in_sender_reserve_on_ahr_before =
		<AssetHubRococo as Chain>::account_data_of(sov_of_sender_on_ah.clone()).free;
	let wnds_in_sender_reserve_on_ahr_before = AssetHubRococo::execute_with(|| {
		type Assets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			wnd_at_rococo_parachains.clone().try_into().unwrap(),
			&sov_of_sender_on_ah,
		)
	});
	let rocs_in_receiver_reserve_on_ahr_before =
		<AssetHubRococo as Chain>::account_data_of(sov_of_receiver_on_ah.clone()).free;
	let wnds_in_receiver_reserve_on_ahr_before = AssetHubRococo::execute_with(|| {
		type Assets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			wnd_at_rococo_parachains.clone().try_into().unwrap(),
			&sov_of_receiver_on_ah,
		)
	});
	let receiver_rocs_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_location.clone(), &receiver)
	});
	let receiver_wnds_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_rococo_parachains.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<PenpalA>(para_to_para_through_hop_sender_assertions);
	test.set_assertion::<AssetHubRococo>(para_to_para_assethub_hop_assertions);
	test.set_assertion::<PenpalB>(para_to_para_through_hop_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_para_transfer_assets_through_ah);
	test.assert();

	// Query final balances
	let sender_rocs_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_location.clone(), &sender)
	});
	let sender_wnds_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_rococo_parachains.clone(), &sender)
	});
	let wnds_in_sender_reserve_on_ahr_after = AssetHubRococo::execute_with(|| {
		type Assets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			wnd_at_rococo_parachains.clone().try_into().unwrap(),
			&sov_of_sender_on_ah,
		)
	});
	let rocs_in_sender_reserve_on_ahr_after =
		<AssetHubRococo as Chain>::account_data_of(sov_of_sender_on_ah).free;
	let wnds_in_receiver_reserve_on_ahr_after = AssetHubRococo::execute_with(|| {
		type Assets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			wnd_at_rococo_parachains.clone().try_into().unwrap(),
			&sov_of_receiver_on_ah,
		)
	});
	let rocs_in_receiver_reserve_on_ahr_after =
		<AssetHubRococo as Chain>::account_data_of(sov_of_receiver_on_ah).free;
	let receiver_rocs_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_location, &receiver)
	});
	let receiver_wnds_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_rococo_parachains, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_rocs_after < sender_rocs_before - roc_to_send);
	assert_eq!(sender_wnds_after, sender_wnds_before - wnd_to_send);
	// Sovereign accounts on reserve are changed accordingly
	assert_eq!(
		rocs_in_sender_reserve_on_ahr_after,
		rocs_in_sender_reserve_on_ahr_before - roc_to_send
	);
	assert_eq!(
		wnds_in_sender_reserve_on_ahr_after,
		wnds_in_sender_reserve_on_ahr_before - wnd_to_send
	);
	assert!(rocs_in_receiver_reserve_on_ahr_after > rocs_in_receiver_reserve_on_ahr_before);
	assert_eq!(
		wnds_in_receiver_reserve_on_ahr_after,
		wnds_in_receiver_reserve_on_ahr_before + wnd_to_send
	);
	// Receiver's balance is increased
	assert!(receiver_rocs_after > receiver_rocs_before);
	assert_eq!(receiver_wnds_after, receiver_wnds_before + wnd_to_send);
}

// ==============================================================================================
// ==== Bidirectional Transfer - Native + Teleportable Foreign Assets - Parachain<->AssetHub ====
// ==============================================================================================
/// Transfers of native asset plus teleportable foreign asset from Parachain to AssetHub and back
/// with fees paid using native asset.
#[test]
fn bidirectional_teleport_foreign_asset_between_para_and_asset_hub_using_explicit_transfer_types() {
	do_bidirectional_teleport_foreign_assets_between_para_and_asset_hub_using_xt(
		para_to_asset_hub_teleport_foreign_assets,
		asset_hub_to_para_teleport_foreign_assets,
	);
}

// ===============================================================
// ===== Transfer - Native Asset - Relay->AssetHub->Parachain ====
// ===============================================================
/// Transfers of native asset Relay to Parachain (using AssetHub reserve). Parachains want to avoid
/// managing SAs on all system chains, thus want all their DOT-in-reserve to be held in their
/// Sovereign Account on Asset Hub.
#[test]
fn transfer_native_asset_from_relay_to_para_through_asset_hub() {
	// Init values for Relay
	let destination = Rococo::child_location_of(PenpalA::para_id());
	let sender = RococoSender::get();
	let amount_to_send: Balance = ROCOCO_ED * 1000;

	// Init values for Parachain
	let relay_native_asset_location = RelayLocation::get();
	let receiver = PenpalAReceiver::get();

	// Init Test
	let test_args = TestContext {
		sender,
		receiver: receiver.clone(),
		args: TestArgs::new_relay(destination.clone(), receiver.clone(), amount_to_send),
	};
	let mut test = RelayToParaThroughAHTest::new(test_args);

	let sov_penpal_on_ah = AssetHubRococo::sovereign_account_id_of(
		AssetHubRococo::sibling_location_of(PenpalA::para_id()),
	);
	// Query initial balances
	let sender_balance_before = test.sender.balance;
	let sov_penpal_on_ah_before = AssetHubRococo::execute_with(|| {
		<AssetHubRococo as AssetHubRococoPallet>::Balances::free_balance(sov_penpal_on_ah.clone())
	});
	let receiver_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &receiver)
	});

	fn relay_assertions(t: RelayToParaThroughAHTest) {
		type RuntimeEvent = <Rococo as Chain>::RuntimeEvent;
		Rococo::assert_xcm_pallet_attempted_complete(None);
		assert_expected_events!(
			Rococo,
			vec![
				// Amount to teleport is withdrawn from Sender
				RuntimeEvent::Balances(pallet_balances::Event::Burned { who, amount }) => {
					who: *who == t.sender.account_id,
					amount: *amount == t.args.amount,
				},
				// Amount to teleport is deposited in Relay's `CheckAccount`
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, amount }) => {
					who: *who == <Rococo as RococoPallet>::XcmPallet::check_account(),
					amount:  *amount == t.args.amount,
				},
			]
		);
	}
	fn asset_hub_assertions(_: RelayToParaThroughAHTest) {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;
		let sov_penpal_on_ah = AssetHubRococo::sovereign_account_id_of(
			AssetHubRococo::sibling_location_of(PenpalA::para_id()),
		);
		assert_expected_events!(
			AssetHubRococo,
			vec![
				// Deposited to receiver parachain SA
				RuntimeEvent::Balances(
					pallet_balances::Event::Minted { who, .. }
				) => {
					who: *who == sov_penpal_on_ah,
				},
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	}
	fn penpal_assertions(t: RelayToParaThroughAHTest) {
		type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
		// Assets in t are relative to the relay chain. The asset here should be relative to
		// Penpal, so parents: 1.
		let expected_id: Location = Location { parents: 1, interior: Here };

		assert_expected_events!(
			PenpalA,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == expected_id,
					owner: *owner == t.receiver.account_id,
				},
			]
		);
	}
	fn transfer_assets_dispatchable(t: RelayToParaThroughAHTest) -> DispatchResult {
		let fee_idx = t.args.fee_asset_item as usize;
		let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
		let asset_hub_location = Rococo::child_location_of(AssetHubRococo::para_id());
		let context = RococoUniversalLocation::get();

		// reanchor fees to the view of destination (Penpal)
		let mut remote_fees = fee.clone().reanchored(&t.args.dest, &context).unwrap();
		if let Fungible(ref mut amount) = remote_fees.fun {
			// we already spent some fees along the way, just use half of what we started with
			*amount = *amount / 2;
		}
		let xcm_on_final_dest = Xcm::<()>(vec![
			BuyExecution { fees: remote_fees, weight_limit: t.args.weight_limit.clone() },
			DepositAsset {
				assets: Wild(AllCounted(t.args.assets.len() as u32)),
				beneficiary: t.args.beneficiary,
			},
		]);

		// reanchor final dest (Penpal) to the view of hop (Asset Hub)
		let mut dest = t.args.dest.clone();
		dest.reanchor(&asset_hub_location, &context).unwrap();
		// on Asset Hub, forward assets to Penpal
		let xcm_on_hop = Xcm::<()>(vec![DepositReserveAsset {
			assets: Wild(AllCounted(t.args.assets.len() as u32)),
			dest,
			xcm: xcm_on_final_dest,
		}]);

		Dmp::make_parachain_reachable(AssetHubRococo::para_id());

		// First leg is a teleport, from there a local-reserve-transfer to final dest
		<Rococo as RococoPallet>::XcmPallet::transfer_assets_using_type_and_then(
			t.signed_origin,
			bx!(asset_hub_location.into()),
			bx!(t.args.assets.into()),
			bx!(TransferType::Teleport),
			bx!(fee.id.into()),
			bx!(TransferType::Teleport),
			bx!(VersionedXcm::from(xcm_on_hop)),
			t.args.weight_limit,
		)
	}

	// Set assertions and dispatchables
	test.set_assertion::<Rococo>(relay_assertions);
	test.set_assertion::<AssetHubRococo>(asset_hub_assertions);
	test.set_assertion::<PenpalA>(penpal_assertions);
	test.set_dispatchable::<Rococo>(transfer_assets_dispatchable);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let sov_penpal_on_ah_after = AssetHubRococo::execute_with(|| {
		<AssetHubRococo as AssetHubRococoPallet>::Balances::free_balance(sov_penpal_on_ah)
	});
	let receiver_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_balance_after < sender_balance_before - amount_to_send);
	// SA on AH balance is increased
	assert!(sov_penpal_on_ah_after > sov_penpal_on_ah_before);
	// Receiver's asset balance is increased
	assert!(receiver_assets_after > receiver_assets_before);
	// Receiver's asset balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_assets_after < receiver_assets_before + amount_to_send);
}
