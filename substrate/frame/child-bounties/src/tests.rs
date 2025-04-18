// This file is part of Substrate.

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

//! Child-bounties pallet tests.

#![cfg(test)]

use super::*;
use crate as pallet_child_bounties;

use frame_support::{
	assert_noop, assert_ok, derive_impl, parameter_types,
	traits::{
		tokens::{PayFromAccount, UnityAssetBalanceConversion},
		ConstU32, ConstU64, OnInitialize,
	},
	weights::Weight,
	PalletId,
};

use sp_runtime::{
	traits::{BadOrigin, IdentityLookup},
	BuildStorage, Perbill, Permill, TokenError,
};

use super::Event as ChildBountiesEvent;

type Block = frame_system::mocking::MockBlock<Test>;
type BountiesError = pallet_bounties::Error<Test>;

// This function directly jumps to a block number, and calls `on_initialize`.
fn go_to_block(n: u64) {
	<Test as pallet_treasury::Config>::BlockNumberProvider::set_block_number(n);
	<Treasury as OnInitialize<u64>>::on_initialize(n);
}

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Bounties: pallet_bounties,
		Treasury: pallet_treasury,
		ChildBounties: pallet_child_bounties,
	}
);

parameter_types! {
	pub const MaximumBlockWeight: Weight = Weight::from_parts(1024, 0);
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

type Balance = u64;
// must be at least 20 bytes long because of child-bounty account derivation.
type AccountId = sp_core::U256;

fn account_id(id: u8) -> AccountId {
	sp_core::U256::from(id)
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}
parameter_types! {
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub TreasuryAccount: AccountId = Treasury::account_id();
	pub const SpendLimit: Balance = u64::MAX;
}

impl pallet_treasury::Config for Test {
	type PalletId = TreasuryPalletId;
	type Currency = pallet_balances::Pallet<Test>;
	type RejectOrigin = frame_system::EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = ConstU64<2>;
	type Burn = Burn;
	type BurnDestination = ();
	type WeightInfo = ();
	type SpendFunds = Bounties;
	type MaxApprovals = ConstU32<100>;
	type SpendOrigin = frame_system::EnsureRootWithSuccess<Self::AccountId, SpendLimit>;
	type AssetKind = ();
	type Beneficiary = Self::AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = UnityAssetBalanceConversion;
	type PayoutPeriod = ConstU64<10>;
	type BlockNumberProvider = System;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}
parameter_types! {
	// This will be 50% of the bounty fee.
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMax: Balance = 1_000;
	pub const CuratorDepositMin: Balance = 3;

}
impl pallet_bounties::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = ConstU64<80>;
	type BountyDepositPayoutDelay = ConstU64<3>;
	type BountyUpdatePeriod = ConstU64<10>;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMax = CuratorDepositMax;
	type CuratorDepositMin = CuratorDepositMin;
	type BountyValueMinimum = ConstU64<5>;
	type DataDepositPerByte = ConstU64<1>;
	type MaximumReasonLength = ConstU32<300>;
	type WeightInfo = ();
	type ChildBountyManager = ChildBounties;
	type OnSlash = ();
}
impl pallet_child_bounties::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxActiveChildBountyCount = ConstU32<2>;
	type ChildBountyValueMinimum = ConstU64<1>;
	type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		// Total issuance will be 200 with treasury account initialized at ED.
		balances: vec![(account_id(0), 100), (account_id(1), 98), (account_id(2), 1)],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();
	pallet_treasury::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	t.into()
}

fn last_event() -> ChildBountiesEvent<Test> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| if let RuntimeEvent::ChildBounties(inner) = e { Some(inner) } else { None })
		.last()
		.unwrap()
}

#[test]
#[allow(deprecated)]
fn genesis_config_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(Treasury::pot(), 0);
		assert_eq!(Treasury::proposal_count(), 0);
	});
}

#[test]
fn minting_works() {
	new_test_ext().execute_with(|| {
		// Check that accumulate works when we have Some value in Dummy already.
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Treasury::pot(), 100);
	});
}

#[test]
fn add_child_bounty() {
	new_test_ext().execute_with(|| {
		// TestProcedure.
		// 1, Create bounty & move to active state with enough bounty fund & parent curator.
		// 2, Parent curator adds child-bounty child-bounty-1, test for error like RequireCurator
		//    ,InsufficientProposersBalance, InsufficientBountyBalance with invalid arguments.
		// 3, Parent curator adds child-bounty child-bounty-1, moves to "Approved" state &
		//    test for the event Added.
		// 4, Test for DB state of `Bounties` & `ChildBounties`.
		// 5, Observe fund transaction moment between Bounty, Child-bounty,
		//    Curator, child-bounty curator & beneficiary.

		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));

		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		let fee = 8;
		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), fee));

		Balances::make_free_balance_be(&account_id(4), 10);

		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// This verifies that the accept curator logic took a deposit.
		let expected_deposit = CuratorDepositMultiplier::get() * fee;
		assert_eq!(Balances::reserved_balance(&account_id(4)), expected_deposit);
		assert_eq!(Balances::free_balance(&account_id(4)), 10 - expected_deposit);

		// Add child-bounty.
		// Acc-4 is the parent curator.
		// Call from invalid origin & check for error "RequireCurator".
		assert_noop!(
			ChildBounties::add_child_bounty(
				RuntimeOrigin::signed(account_id(0)),
				0,
				10,
				b"12345-p1".to_vec()
			),
			BountiesError::RequireCurator,
		);

		// Update the parent curator balance.
		Balances::make_free_balance_be(&account_id(4), 101);

		// parent curator fee is reserved on parent bounty account.
		assert_eq!(Balances::free_balance(Bounties::bounty_account_id(0)), 50);
		assert_eq!(Balances::reserved_balance(Bounties::bounty_account_id(0)), 0);

		assert_noop!(
			ChildBounties::add_child_bounty(
				RuntimeOrigin::signed(account_id(4)),
				0,
				50,
				b"12345-p1".to_vec()
			),
			TokenError::NotExpendable,
		);

		assert_noop!(
			ChildBounties::add_child_bounty(
				RuntimeOrigin::signed(account_id(4)),
				0,
				100,
				b"12345-p1".to_vec()
			),
			Error::<Test>::InsufficientBountyBalance,
		);

		// Add child-bounty with valid value, which can be funded by parent bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));

		// Check for the event child-bounty added.
		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		assert_eq!(Balances::free_balance(account_id(4)), 101);
		assert_eq!(Balances::reserved_balance(account_id(4)), expected_deposit);

		// DB check.
		// Check the child-bounty status.
		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee: 0,
				curator_deposit: 0,
				status: ChildBountyStatus::Added,
			}
		);

		// Check the child-bounty count.
		assert_eq!(pallet_child_bounties::ParentChildBounties::<Test>::get(0), 1);

		// Check the child-bounty description status.
		assert_eq!(
			pallet_child_bounties::ChildBountyDescriptionsV1::<Test>::get(0, 0).unwrap(),
			b"12345-p1".to_vec(),
		);
	});
}

#[test]
fn child_bounty_assign_curator() {
	new_test_ext().execute_with(|| {
		// TestProcedure
		// 1, Create bounty & move to active state with enough bounty fund & parent curator.
		// 2, Parent curator adds child-bounty child-bounty-1, moves to "Active" state.
		// 3, Test for DB state of `ChildBounties`.

		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		Balances::make_free_balance_be(&account_id(4), 101);
		Balances::make_free_balance_be(&account_id(8), 101);

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));

		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		let fee = 4;
		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), fee));
		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Bounty account status before adding child-bounty.
		assert_eq!(Balances::free_balance(Bounties::bounty_account_id(0)), 50);
		assert_eq!(Balances::reserved_balance(Bounties::bounty_account_id(0)), 0);

		// Check the balance of parent curator.
		// Curator deposit is reserved for parent curator on parent bounty.
		let expected_deposit = Bounties::calculate_curator_deposit(&fee);
		assert_eq!(Balances::free_balance(account_id(4)), 101 - expected_deposit);
		assert_eq!(Balances::reserved_balance(account_id(4)), expected_deposit);

		// Add child-bounty.
		// Acc-4 is the parent curator & make sure enough deposit.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));

		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		// Bounty account status after adding child-bounty.
		assert_eq!(Balances::free_balance(Bounties::bounty_account_id(0)), 40);
		assert_eq!(Balances::reserved_balance(Bounties::bounty_account_id(0)), 0);

		// Child-bounty account status.
		assert_eq!(Balances::free_balance(ChildBounties::child_bounty_account_id(0, 0)), 10);
		assert_eq!(Balances::reserved_balance(ChildBounties::child_bounty_account_id(0, 0)), 0);

		let fee = 6u64;
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(8),
			fee
		));

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: 0,
				status: ChildBountyStatus::CuratorProposed { curator: account_id(8) },
			}
		);

		// Check the balance of parent curator.
		assert_eq!(Balances::free_balance(account_id(4)), 101 - expected_deposit);
		assert_eq!(Balances::reserved_balance(account_id(4)), expected_deposit);

		assert_noop!(
			ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(3)), 0, 0),
			BountiesError::RequireCurator,
		);

		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(8)), 0, 0));

		let expected_child_deposit = CuratorDepositMultiplier::get() * fee;

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: expected_child_deposit,
				status: ChildBountyStatus::Active { curator: account_id(8) },
			}
		);

		// Deposit for child-bounty curator deposit is reserved.
		assert_eq!(Balances::free_balance(account_id(8)), 101 - expected_child_deposit);
		assert_eq!(Balances::reserved_balance(account_id(8)), expected_child_deposit);

		// Bounty account status at exit.
		assert_eq!(Balances::free_balance(Bounties::bounty_account_id(0)), 40);
		assert_eq!(Balances::reserved_balance(Bounties::bounty_account_id(0)), 0);

		// Child-bounty account status at exit.
		assert_eq!(Balances::free_balance(ChildBounties::child_bounty_account_id(0, 0)), 10);
		assert_eq!(Balances::reserved_balance(ChildBounties::child_bounty_account_id(0, 0)), 0);

		// Treasury account status at exit.
		assert_eq!(Balances::free_balance(Treasury::account_id()), 26);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);
	});
}

#[test]
fn award_claim_child_bounty() {
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));

		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), 6));
		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));

		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		// Propose and accept curator for child-bounty.
		let fee = 8;
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(8),
			fee
		));
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(8)), 0, 0));

		// Award child-bounty.
		// Test for non child-bounty curator.
		assert_noop!(
			ChildBounties::award_child_bounty(
				RuntimeOrigin::signed(account_id(3)),
				0,
				0,
				account_id(7)
			),
			BountiesError::RequireCurator,
		);

		assert_ok!(ChildBounties::award_child_bounty(
			RuntimeOrigin::signed(account_id(8)),
			0,
			0,
			account_id(7)
		));

		let expected_deposit = CuratorDepositMultiplier::get() * fee;
		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: expected_deposit,
				status: ChildBountyStatus::PendingPayout {
					curator: account_id(8),
					beneficiary: account_id(7),
					unlock_at: 5
				},
			}
		);

		// Claim child-bounty.
		// Test for Premature condition.
		assert_noop!(
			ChildBounties::claim_child_bounty(RuntimeOrigin::signed(account_id(7)), 0, 0),
			BountiesError::Premature
		);

		go_to_block(9);

		assert_ok!(ChildBounties::claim_child_bounty(RuntimeOrigin::signed(account_id(7)), 0, 0));

		// Ensure child-bounty curator is paid with curator fee & deposit refund.
		assert_eq!(Balances::free_balance(account_id(8)), 101 + fee);
		assert_eq!(Balances::reserved_balance(account_id(8)), 0);

		// Ensure executor is paid with beneficiary amount.
		assert_eq!(Balances::free_balance(account_id(7)), 10 - fee);
		assert_eq!(Balances::reserved_balance(account_id(7)), 0);

		// Child-bounty account status.
		assert_eq!(Balances::free_balance(ChildBounties::child_bounty_account_id(0, 0)), 0);
		assert_eq!(Balances::reserved_balance(ChildBounties::child_bounty_account_id(0, 0)), 0);

		// Check the child-bounty count.
		assert_eq!(pallet_child_bounties::ParentChildBounties::<Test>::get(0), 0);
	});
}

#[test]
fn close_child_bounty_added() {
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));

		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), 6));

		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));

		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		go_to_block(4);

		// Close child-bounty.
		// Wrong origin.
		assert_noop!(
			ChildBounties::close_child_bounty(RuntimeOrigin::signed(account_id(7)), 0, 0),
			BadOrigin
		);
		assert_noop!(
			ChildBounties::close_child_bounty(RuntimeOrigin::signed(account_id(8)), 0, 0),
			BadOrigin
		);

		// Correct origin - parent curator.
		assert_ok!(ChildBounties::close_child_bounty(RuntimeOrigin::signed(account_id(4)), 0, 0));

		// Check the child-bounty count.
		assert_eq!(pallet_child_bounties::ParentChildBounties::<Test>::get(0), 0);

		// Parent-bounty account status.
		assert_eq!(Balances::free_balance(Bounties::bounty_account_id(0)), 50);
		assert_eq!(Balances::reserved_balance(Bounties::bounty_account_id(0)), 0);

		// Child-bounty account status.
		assert_eq!(Balances::free_balance(ChildBounties::child_bounty_account_id(0, 0)), 0);
		assert_eq!(Balances::reserved_balance(ChildBounties::child_bounty_account_id(0, 0)), 0);
	});
}

#[test]
fn close_child_bounty_active() {
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));

		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), 6));

		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));

		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		// Propose and accept curator for child-bounty.
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(8),
			2
		));
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(8)), 0, 0));

		// Close child-bounty in active state.
		assert_ok!(ChildBounties::close_child_bounty(RuntimeOrigin::signed(account_id(4)), 0, 0));

		// Check the child-bounty count.
		assert_eq!(pallet_child_bounties::ParentChildBounties::<Test>::get(0), 0);

		// Ensure child-bounty curator balance is unreserved.
		assert_eq!(Balances::free_balance(account_id(8)), 101);
		assert_eq!(Balances::reserved_balance(account_id(8)), 0);

		// Parent-bounty account status.
		assert_eq!(Balances::free_balance(Bounties::bounty_account_id(0)), 50);
		assert_eq!(Balances::reserved_balance(Bounties::bounty_account_id(0)), 0);

		// Child-bounty account status.
		assert_eq!(Balances::free_balance(ChildBounties::child_bounty_account_id(0, 0)), 0);
		assert_eq!(Balances::reserved_balance(ChildBounties::child_bounty_account_id(0, 0)), 0);
	});
}

#[test]
fn close_child_bounty_pending() {
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));

		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		let parent_fee = 6;
		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), parent_fee));
		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));

		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		// Propose and accept curator for child-bounty.
		let child_fee = 4;
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(8),
			child_fee
		));
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(8)), 0, 0));
		let expected_child_deposit = CuratorDepositMin::get();

		assert_ok!(ChildBounties::award_child_bounty(
			RuntimeOrigin::signed(account_id(8)),
			0,
			0,
			account_id(7)
		));

		// Close child-bounty in pending_payout state.
		assert_noop!(
			ChildBounties::close_child_bounty(RuntimeOrigin::signed(account_id(4)), 0, 0),
			BountiesError::PendingPayout
		);

		// Check the child-bounty count.
		assert_eq!(pallet_child_bounties::ParentChildBounties::<Test>::get(0), 1);

		// Ensure no changes in child-bounty curator balance.
		assert_eq!(Balances::reserved_balance(account_id(8)), expected_child_deposit);
		assert_eq!(Balances::free_balance(account_id(8)), 101 - expected_child_deposit);

		// Child-bounty account status.
		assert_eq!(Balances::free_balance(ChildBounties::child_bounty_account_id(0, 0)), 10);
		assert_eq!(Balances::reserved_balance(ChildBounties::child_bounty_account_id(0, 0)), 0);
	});
}

#[test]
fn child_bounty_added_unassign_curator() {
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));

		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), 6));

		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));

		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		// Unassign curator in added state.
		assert_noop!(
			ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(4)), 0, 0),
			BountiesError::UnexpectedStatus
		);
	});
}

#[test]
fn child_bounty_curator_proposed_unassign_curator() {
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));

		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), 6));

		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));

		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		// Propose curator for child-bounty.
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(8),
			2
		));

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee: 2,
				curator_deposit: 0,
				status: ChildBountyStatus::CuratorProposed { curator: account_id(8) },
			}
		);

		// Random account cannot unassign the curator when in proposed state.
		assert_noop!(
			ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(99)), 0, 0),
			BadOrigin
		);

		// Unassign curator.
		assert_ok!(ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(4)), 0, 0));

		// Verify updated child-bounty status.
		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee: 2,
				curator_deposit: 0,
				status: ChildBountyStatus::Added,
			}
		);
	});
}

#[test]
fn child_bounty_active_unassign_curator() {
	// Covers all scenarios with all origin types.
	// Step 1: Setup bounty, child bounty.
	// Step 2: Assign, accept curator for child bounty. Unassign from reject origin. Should slash.
	// Step 3: Assign, accept another curator for child bounty. Unassign from parent-bounty curator.
	// Should slash. Step 4: Assign, accept another curator for child bounty. Unassign from
	// child-bounty curator. Should NOT slash. Step 5: Assign, accept another curator for child
	// bounty. Unassign from random account. Should slash.
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator.
		Balances::make_free_balance_be(&account_id(6), 101); // Child-bounty curator 1.
		Balances::make_free_balance_be(&account_id(7), 101); // Child-bounty curator 2.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator 3.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));

		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), 6));
		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Create Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));
		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		go_to_block(3);

		// Propose and accept curator for child-bounty.
		let fee = 6;
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(8),
			fee
		));
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(8)), 0, 0));
		let expected_child_deposit = CuratorDepositMultiplier::get() * fee;

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: expected_child_deposit,
				status: ChildBountyStatus::Active { curator: account_id(8) },
			}
		);

		go_to_block(4);

		// Unassign curator - from reject origin.
		assert_ok!(ChildBounties::unassign_curator(RuntimeOrigin::root(), 0, 0));

		// Verify updated child-bounty status.
		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: 0,
				status: ChildBountyStatus::Added,
			}
		);

		// Ensure child-bounty curator was slashed.
		assert_eq!(Balances::free_balance(account_id(8)), 101 - expected_child_deposit);
		assert_eq!(Balances::reserved_balance(account_id(8)), 0); // slashed

		// Propose and accept curator for child-bounty again.
		let fee = 2;
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(7),
			fee
		));
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(7)), 0, 0));
		let expected_child_deposit = CuratorDepositMin::get();

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: expected_child_deposit,
				status: ChildBountyStatus::Active { curator: account_id(7) },
			}
		);

		go_to_block(5);

		// Unassign curator again - from parent curator.
		assert_ok!(ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(4)), 0, 0));

		// Verify updated child-bounty status.
		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee: 2,
				curator_deposit: 0,
				status: ChildBountyStatus::Added,
			}
		);

		// Ensure child-bounty curator was slashed.
		assert_eq!(Balances::free_balance(account_id(7)), 101 - expected_child_deposit);
		assert_eq!(Balances::reserved_balance(account_id(7)), 0); // slashed

		// Propose and accept curator for child-bounty again.
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(6),
			2
		));
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(6)), 0, 0));

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: expected_child_deposit,
				status: ChildBountyStatus::Active { curator: account_id(6) },
			}
		);

		go_to_block(6);

		// Unassign curator again - from child-bounty curator.
		assert_ok!(ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(6)), 0, 0));

		// Verify updated child-bounty status.
		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee: 2,
				curator_deposit: 0,
				status: ChildBountyStatus::Added,
			}
		);

		// Ensure child-bounty curator was **not** slashed.
		assert_eq!(Balances::free_balance(account_id(6)), 101); // not slashed
		assert_eq!(Balances::reserved_balance(account_id(6)), 0);

		// Propose and accept curator for child-bounty one last time.
		let fee = 2;
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(6),
			fee
		));
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(6)), 0, 0));
		let expected_child_deposit = CuratorDepositMin::get();

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: expected_child_deposit,
				status: ChildBountyStatus::Active { curator: account_id(6) },
			}
		);

		go_to_block(7);

		// Unassign curator again - from non curator; non reject origin; some random guy.
		// Bounty update period is not yet complete.
		assert_noop!(
			ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(3)), 0, 0),
			BountiesError::Premature
		);

		go_to_block(20);

		// Unassign child curator from random account after inactivity.
		assert_ok!(ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(3)), 0, 0));

		// Verify updated child-bounty status.
		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee: 2,
				curator_deposit: 0,
				status: ChildBountyStatus::Added,
			}
		);

		// Ensure child-bounty curator was slashed.
		assert_eq!(Balances::free_balance(account_id(6)), 101 - expected_child_deposit); // slashed
		assert_eq!(Balances::reserved_balance(account_id(6)), 0);
	});
}

#[test]
fn parent_bounty_inactive_unassign_curator_child_bounty() {
	// Unassign curator when parent bounty in not in active state.
	// This can happen when the curator of parent bounty has been unassigned.
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator 1.
		Balances::make_free_balance_be(&account_id(5), 101); // Parent-bounty curator 2.
		Balances::make_free_balance_be(&account_id(6), 101); // Child-bounty curator 1.
		Balances::make_free_balance_be(&account_id(7), 101); // Child-bounty curator 2.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator 3.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));
		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), 6));
		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Create Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));
		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		go_to_block(3);

		// Propose and accept curator for child-bounty.
		let fee = 8;
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(8),
			fee
		));
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(8)), 0, 0));
		let expected_child_deposit = CuratorDepositMultiplier::get() * fee;

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: expected_child_deposit,
				status: ChildBountyStatus::Active { curator: account_id(8) },
			}
		);

		go_to_block(4);

		// Unassign parent bounty curator.
		assert_ok!(Bounties::unassign_curator(RuntimeOrigin::root(), 0));

		go_to_block(5);

		// Try unassign child-bounty curator - from non curator; non reject
		// origin; some random guy. Bounty update period is not yet complete.
		assert_noop!(
			ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(3)), 0, 0),
			Error::<Test>::ParentBountyNotActive
		);

		// Unassign curator - from reject origin.
		assert_ok!(ChildBounties::unassign_curator(RuntimeOrigin::root(), 0, 0));

		// Verify updated child-bounty status.
		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: 0,
				status: ChildBountyStatus::Added,
			}
		);

		// Ensure child-bounty curator was slashed.
		assert_eq!(Balances::free_balance(account_id(8)), 101 - expected_child_deposit);
		assert_eq!(Balances::reserved_balance(account_id(8)), 0); // slashed

		go_to_block(6);

		// Propose and accept curator for parent-bounty again.
		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(5), 6));
		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(5)), 0));

		go_to_block(7);

		// Propose and accept curator for child-bounty again.
		let fee = 2;
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(5)),
			0,
			0,
			account_id(7),
			fee
		));
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(7)), 0, 0));
		let expected_deposit = CuratorDepositMin::get();

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: expected_deposit,
				status: ChildBountyStatus::Active { curator: account_id(7) },
			}
		);

		go_to_block(8);

		assert_noop!(
			ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(3)), 0, 0),
			BountiesError::Premature
		);

		// Unassign parent bounty curator again.
		assert_ok!(Bounties::unassign_curator(RuntimeOrigin::signed(account_id(5)), 0));

		go_to_block(9);

		// Unassign curator again - from parent curator.
		assert_ok!(ChildBounties::unassign_curator(RuntimeOrigin::signed(account_id(7)), 0, 0));

		// Verify updated child-bounty status.
		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee: 2,
				curator_deposit: 0,
				status: ChildBountyStatus::Added,
			}
		);

		// Ensure child-bounty curator was not slashed.
		assert_eq!(Balances::free_balance(account_id(7)), 101);
		assert_eq!(Balances::reserved_balance(account_id(7)), 0); // slashed
	});
}

#[test]
fn close_parent_with_child_bounty() {
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));
		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		// Try add child-bounty.
		// Should fail, parent bounty not active yet.
		assert_noop!(
			ChildBounties::add_child_bounty(
				RuntimeOrigin::signed(account_id(4)),
				0,
				10,
				b"12345-p1".to_vec()
			),
			Error::<Test>::ParentBountyNotActive
		);

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), 6));
		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));
		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		go_to_block(4);

		// Try close parent-bounty.
		// Child bounty active, can't close parent.
		assert_noop!(
			Bounties::close_bounty(RuntimeOrigin::root(), 0),
			BountiesError::HasActiveChildBounty
		);

		// Close child-bounty.
		assert_ok!(ChildBounties::close_child_bounty(RuntimeOrigin::root(), 0, 0));

		// Check the child-bounty count.
		assert_eq!(pallet_child_bounties::ParentChildBounties::<Test>::get(0), 0);
		assert_eq!(pallet_child_bounties::ParentTotalChildBounties::<Test>::get(0), 1);

		// Try close parent-bounty again.
		// Should pass this time.
		assert_ok!(Bounties::close_bounty(RuntimeOrigin::root(), 0));

		// Check the total count is removed after the parent bounty removal.
		assert_eq!(pallet_child_bounties::ParentTotalChildBounties::<Test>::get(0), 0);
	});
}

#[test]
fn children_curator_fee_calculation_test() {
	// Tests the calculation of subtracting child-bounty curator fee
	// from parent bounty fee when claiming bounties.
	new_test_ext().execute_with(|| {
		// Make the parent bounty.
		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), 101);
		assert_eq!(Balances::free_balance(Treasury::account_id()), 101);
		assert_eq!(Balances::reserved_balance(Treasury::account_id()), 0);

		// Bounty curator initial balance.
		Balances::make_free_balance_be(&account_id(4), 101); // Parent-bounty curator.
		Balances::make_free_balance_be(&account_id(8), 101); // Child-bounty curator.

		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(account_id(0)),
			50,
			b"12345".to_vec()
		));
		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), 0));

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(RuntimeOrigin::root(), 0, account_id(4), 6));
		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(account_id(4)), 0));

		// Child-bounty.
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(account_id(4)),
			0,
			10,
			b"12345-p1".to_vec()
		));
		assert_eq!(last_event(), ChildBountiesEvent::Added { index: 0, child_index: 0 });

		go_to_block(4);

		let fee = 6;

		// Propose curator for child-bounty.
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(account_id(4)),
			0,
			0,
			account_id(8),
			fee
		));
		// Check curator fee added to the sum.
		assert_eq!(pallet_child_bounties::ChildrenCuratorFees::<Test>::get(0), fee);
		// Accept curator for child-bounty.
		assert_ok!(ChildBounties::accept_curator(RuntimeOrigin::signed(account_id(8)), 0, 0));
		// Award child-bounty.
		assert_ok!(ChildBounties::award_child_bounty(
			RuntimeOrigin::signed(account_id(8)),
			0,
			0,
			account_id(7)
		));

		let expected_child_deposit = CuratorDepositMultiplier::get() * fee;

		assert_eq!(
			pallet_child_bounties::ChildBounties::<Test>::get(0, 0).unwrap(),
			ChildBounty {
				parent_bounty: 0,
				value: 10,
				fee,
				curator_deposit: expected_child_deposit,
				status: ChildBountyStatus::PendingPayout {
					curator: account_id(8),
					beneficiary: account_id(7),
					unlock_at: 7
				},
			}
		);

		go_to_block(9);

		// Claim child-bounty.
		assert_ok!(ChildBounties::claim_child_bounty(RuntimeOrigin::signed(account_id(7)), 0, 0));

		// Check the child-bounty count.
		assert_eq!(pallet_child_bounties::ParentChildBounties::<Test>::get(0), 0);

		// Award the parent bounty.
		assert_ok!(Bounties::award_bounty(RuntimeOrigin::signed(account_id(4)), 0, account_id(9)));

		go_to_block(15);

		// Check the total count.
		assert_eq!(pallet_child_bounties::ParentTotalChildBounties::<Test>::get(0), 1);

		// Claim the parent bounty.
		assert_ok!(Bounties::claim_bounty(RuntimeOrigin::signed(account_id(9)), 0));

		// Check the total count after the parent bounty removal.
		assert_eq!(pallet_child_bounties::ParentTotalChildBounties::<Test>::get(0), 0);

		// Ensure parent-bounty curator received correctly reduced fee.
		assert_eq!(Balances::free_balance(account_id(4)), 101 + 6 - fee); // 101 + 6 - 2
		assert_eq!(Balances::reserved_balance(account_id(4)), 0);

		// Verify parent-bounty beneficiary balance.
		assert_eq!(Balances::free_balance(account_id(9)), 34);
		assert_eq!(Balances::reserved_balance(account_id(9)), 0);
	});
}

#[test]
fn accept_curator_handles_different_deposit_calculations() {
	// This test will verify that a bounty with and without a fee results
	// in a different curator deposit, and if the child curator matches the parent curator.
	new_test_ext().execute_with(|| {
		// Setup a parent bounty.
		let parent_curator = account_id(0);
		let parent_index = 0;
		let parent_value = 1_000_000;
		let parent_fee = 10_000;

		go_to_block(1);
		Balances::make_free_balance_be(&Treasury::account_id(), parent_value * 3);
		Balances::make_free_balance_be(&parent_curator, parent_fee * 100);
		assert_ok!(Bounties::propose_bounty(
			RuntimeOrigin::signed(parent_curator),
			parent_value,
			b"12345".to_vec()
		));
		assert_ok!(Bounties::approve_bounty(RuntimeOrigin::root(), parent_index));

		go_to_block(2);

		assert_ok!(Bounties::propose_curator(
			RuntimeOrigin::root(),
			parent_index,
			parent_curator,
			parent_fee
		));
		assert_ok!(Bounties::accept_curator(RuntimeOrigin::signed(parent_curator), parent_index));

		// Now we can start creating some child bounties.
		// Case 1: Parent and child curator are not the same.

		let child_index = 0;
		let child_curator = account_id(1);
		let child_value = 1_000;
		let child_fee = 100;
		let starting_balance = 100 * child_fee + child_value;

		Balances::make_free_balance_be(&child_curator, starting_balance);
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(parent_curator),
			parent_index,
			child_value,
			b"12345-p1".to_vec()
		));
		go_to_block(3);
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(parent_curator),
			parent_index,
			child_index,
			child_curator,
			child_fee
		));
		assert_ok!(ChildBounties::accept_curator(
			RuntimeOrigin::signed(child_curator),
			parent_index,
			child_index
		));

		let expected_deposit = CuratorDepositMultiplier::get() * child_fee;
		assert_eq!(Balances::free_balance(child_curator), starting_balance - expected_deposit);
		assert_eq!(Balances::reserved_balance(child_curator), expected_deposit);

		// Case 2: Parent and child curator are the same.

		let child_index = 1;
		let child_curator = parent_curator; // The same as parent bounty curator
		let child_value = 1_000;
		let child_fee = 10;

		let free_before = Balances::free_balance(&parent_curator);
		let reserved_before = Balances::reserved_balance(&parent_curator);

		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(parent_curator),
			parent_index,
			child_value,
			b"12345-p1".to_vec()
		));
		go_to_block(4);
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(parent_curator),
			parent_index,
			child_index,
			child_curator,
			child_fee
		));
		assert_ok!(ChildBounties::accept_curator(
			RuntimeOrigin::signed(child_curator),
			parent_index,
			child_index
		));

		// No expected deposit
		assert_eq!(Balances::free_balance(child_curator), free_before);
		assert_eq!(Balances::reserved_balance(child_curator), reserved_before);

		// Case 3: Upper Limit

		let child_index = 2;
		let child_curator = account_id(2);
		let child_value = 10_000;
		let child_fee = 5_000;

		Balances::make_free_balance_be(&child_curator, starting_balance);
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(parent_curator),
			parent_index,
			child_value,
			b"12345-p1".to_vec()
		));
		go_to_block(5);
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(parent_curator),
			parent_index,
			child_index,
			child_curator,
			child_fee
		));
		assert_ok!(ChildBounties::accept_curator(
			RuntimeOrigin::signed(child_curator),
			parent_index,
			child_index
		));

		let expected_deposit = CuratorDepositMax::get();
		assert_eq!(Balances::free_balance(child_curator), starting_balance - expected_deposit);
		assert_eq!(Balances::reserved_balance(child_curator), expected_deposit);

		// There is a max number of child bounties at a time.
		assert_ok!(ChildBounties::impl_close_child_bounty(parent_index, child_index));

		// Case 4: Lower Limit

		let child_index = 3;
		let child_curator = account_id(3);
		let child_value = 10_000;
		let child_fee = 0;

		Balances::make_free_balance_be(&child_curator, starting_balance);
		assert_ok!(ChildBounties::add_child_bounty(
			RuntimeOrigin::signed(parent_curator),
			parent_index,
			child_value,
			b"12345-p1".to_vec()
		));
		go_to_block(5);
		assert_ok!(ChildBounties::propose_curator(
			RuntimeOrigin::signed(parent_curator),
			parent_index,
			child_index,
			child_curator,
			child_fee
		));
		assert_ok!(ChildBounties::accept_curator(
			RuntimeOrigin::signed(child_curator),
			parent_index,
			child_index
		));

		let expected_deposit = CuratorDepositMin::get();
		assert_eq!(Balances::free_balance(child_curator), starting_balance - expected_deposit);
		assert_eq!(Balances::reserved_balance(child_curator), expected_deposit);
	});
}

#[test]
fn integrity_test() {
	new_test_ext().execute_with(|| {
		ChildBounties::integrity_test();
	});
}
